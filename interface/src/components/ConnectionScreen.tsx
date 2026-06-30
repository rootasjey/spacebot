import { useState, useCallback, useEffect, lazy, Suspense } from "react";
import { Button, Input } from "@spacedrive/primitives";
import { useServer } from "@/hooks/useServer";
import {
	dragRegionAttributes,
	IS_DESKTOP,
	invoke as platformInvoke,
} from "@/platform";

const Orb = lazy(() => import("@/components/Orb"));

/**
 * Full-screen connection screen shown when the app cannot reach
 * the spacebot server. Allows changing the server URL and, in
 * desktop mode, configuring the Umbrel password for proxy auth.
 */
export function ConnectionScreen() {
	const { serverUrl, setServerUrl, state } = useServer();
	const [draft, setDraft] = useState(serverUrl);
	const [umbrelPassword, setUmbrelPassword] = useState("");
	const [umbrelPasswordLoaded, setUmbrelPasswordLoaded] = useState(false);

	// Load saved Umbrel password on mount (desktop only)
	useEffect(() => {
		if (!IS_DESKTOP || umbrelPasswordLoaded) return;
		(async () => {
			try {
				const saved = await platformInvoke<string>("get_umbrel_password");
				if (saved) setUmbrelPassword(saved);
			} catch { /* ok */ }
			setUmbrelPasswordLoaded(true);
		})();
	}, [umbrelPasswordLoaded]);

	// Keep draft in sync when serverUrl changes externally
	useEffect(() => {
		setDraft(serverUrl);
	}, [serverUrl]);

	const handleConnect = useCallback(() => {
		setServerUrl(draft);
		// Save Umbrel password if set
		if (IS_DESKTOP && umbrelPassword) {
			platformInvoke("set_umbrel_password", {
				password: umbrelPassword,
			}).catch(() => {});
		}
	}, [draft, setServerUrl, umbrelPassword]);

	const handleKeyDown = useCallback(
		(event: React.KeyboardEvent) => {
			if (event.key === "Enter") handleConnect();
		},
		[handleConnect],
	);

	const isChecking = state === "checking";
	const isRemoteUrl = draft.includes("//") && !draft.includes("localhost") && !draft.includes("127.0.0.1");

	return (
		<div className="flex h-screen w-full flex-col items-center justify-center bg-app overflow-hidden">
			{/* Draggable titlebar region for the desktop host */}
			{IS_DESKTOP && (
				<div
					{...dragRegionAttributes()}
					className="fixed inset-x-0 top-0 h-8"
				/>
			)}

			<div className="flex w-full max-w-md flex-col items-center gap-8 px-6">
				{/* Orb + Title */}
				<div className="flex flex-col items-center gap-3">
					<div className="relative h-[160px] w-[160px]">
						<div className="absolute inset-[calc(5%-10px)] z-0">
							<img
								src="/ball.png"
								alt="Spacebot"
								className="h-full w-full object-contain"
							/>
						</div>
						<div className="absolute inset-0 z-10">
							<Suspense fallback={null}>
								<Orb
									hue={-30}
									hoverIntensity={0}
									rotateOnHover
								/>
							</Suspense>
						</div>
					</div>
					<h1 className="font-plex text-xl font-semibold text-ink">
						Connect to Spacebot
					</h1>
					<p className="text-center text-sm text-ink-dull">
						Enter the URL of your Spacebot instance.
					</p>
				</div>

				{/* URL Input */}
				<div className="flex w-full flex-col gap-3">
					<label className="text-xs font-medium text-ink-dull">
						Server URL
					</label>
					<div className="flex gap-2">
						<Input
							value={draft}
							onChange={(event) => setDraft(event.target.value)}
							onKeyDown={handleKeyDown}
							placeholder="http://localhost:19898"
							className="flex-1"
							size="md"
							disabled={isChecking}
						/>
						<Button
							onClick={handleConnect}
							disabled={isChecking || !draft.trim()}
							size="md"
							variant="accent"
							className="bg-[hsl(282,70%,57%)] text-white shadow hover:bg-[hsl(282,70%,50%)] hover:text-white"
						>
							Connect
						</Button>
					</div>

					{/* Umbrel password (desktop + remote URL only) */}
					{IS_DESKTOP && isRemoteUrl && (
						<div className="flex flex-col gap-1.5">
							<label className="text-xs font-medium text-ink-dull">
								Umbrel Password
								<span className="ml-1 text-ink-faint">
									(required if behind Umbrel auth)
								</span>
							</label>
							<Input
								type="password"
								value={umbrelPassword}
								onChange={(e) => setUmbrelPassword(e.target.value)}
								placeholder="Umbrel password"
								size="md"
							/>
						</div>
					)}

					{/* Connection status */}
					{isChecking ? (
						<p className="text-xs text-ink-faint">
							Connecting...
						</p>
					) : state === "disconnected" ? (
						<p className="text-xs text-ink-faint">
							Not connected
						</p>
					) : null}
				</div>

				{/* Footer hint */}
				<p className="text-center text-xs text-ink-faint">
					Spacebot runs on port 19898 by default.
					{" "}Install via{" "}
					<span className="font-mono text-ink-dull">
						docker
					</span>{" "}
					or download from{" "}
					<a
						href="https://spacebot.sh"
						target="_blank"
						rel="noopener noreferrer"
						className="text-accent hover:underline"
					>
						spacebot.sh
					</a>
				</p>
			</div>
		</div>
	);
}
