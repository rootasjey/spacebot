import {
	useTheme,
	THEMES,
	LIGHT_THEMES,
	DARK_THEMES,
	type ThemeId,
} from "@/hooks/useTheme";

export function AppearanceSection() {
	const {
		theme,
		setTheme,
		systemLight,
		setSystemLight,
		systemDark,
		setSystemDark,
	} = useTheme();

	return (
		<div className="mx-auto max-w-2xl px-6 py-6">
			<div className="mb-6">
				<h2 className="font-plex text-sm font-semibold text-ink">Theme</h2>
				<p className="mt-1 text-sm text-ink-dull">
					Choose a theme for the dashboard interface.
				</p>
			</div>

			<div className="grid grid-cols-2 gap-3">
				{THEMES.map((t) => (
					<button
						key={t.id}
						onClick={() => setTheme(t.id)}
						className={`group relative flex flex-col items-start rounded-lg border p-4 text-left transition-colors ${
							theme === t.id
								? "border-accent bg-accent/10"
								: "border-app-line bg-app-box hover:border-app-line/80 hover:bg-app-hover"
						}`}
					>
						<div className="flex w-full items-center justify-between">
							<span className="text-sm font-medium text-ink">{t.name}</span>
							{theme === t.id && (
								<span className="h-2 w-2 rounded-full bg-accent" />
							)}
						</div>
						<p className="mt-1 text-sm text-ink-dull">{t.description}</p>
						<ThemePreview themeId={t.id} />

						{t.id === "system" && theme === "system" && (
							<div className="mt-4 w-full space-y-3" onClick={(e) => e.stopPropagation()}>
								<div>
									<label className="text-xs font-medium text-ink-dull">
										Light theme
									</label>
									<div className="mt-1.5 flex flex-wrap gap-1.5">
										{LIGHT_THEMES.map((lt) => {
											const opt = THEMES.find((th) => th.id === lt)!;
											return (
												<button
													key={lt}
													onClick={() => setSystemLight(lt)}
													className={`rounded-md border px-2.5 py-1 text-xs transition-colors ${
														systemLight === lt
															? "border-accent bg-accent/15 text-accent"
															: "border-app-line text-ink-dull hover:border-app-line/80 hover:text-ink"
													}`}
												>
													{opt.name}
												</button>
											);
										})}
									</div>
								</div>
								<div>
									<label className="text-xs font-medium text-ink-dull">
										Dark theme
									</label>
									<div className="mt-1.5 flex flex-wrap gap-1.5">
										{DARK_THEMES.map((dt) => {
											const opt = THEMES.find((th) => th.id === dt)!;
											return (
												<button
													key={dt}
													onClick={() => setSystemDark(dt)}
													className={`rounded-md border px-2.5 py-1 text-xs transition-colors ${
														systemDark === dt
															? "border-accent bg-accent/15 text-accent"
															: "border-app-line text-ink-dull hover:border-app-line/80 hover:text-ink"
													}`}
												>
													{opt.name}
												</button>
											);
										})}
									</div>
								</div>
							</div>
						)}
					</button>
				))}
			</div>
		</div>
	);
}

const PREVIEW_COLORS: Record<string, {bg: string; sidebar: string; accent: string}> = {
	default: {bg: "#1c1d26", sidebar: "#101118", accent: "#2499ff"},
	vanilla: {bg: "#ffffff", sidebar: "#f5f5f6", accent: "#2499ff"},
	midnight: {bg: "#121428", sidebar: "#0a0b14", accent: "#2499ff"},
	noir: {bg: "#080808", sidebar: "#000000", accent: "#2499ff"},
	slate: {bg: "#151619", sidebar: "#0e0f12", accent: "#2499ff"},
	nord: {bg: "#1a1e27", sidebar: "#11141b", accent: "#2499ff"},
	mocha: {bg: "#1a1614", sidebar: "#110f0d", accent: "#2499ff"},
};

function ThemePreview({themeId}: {themeId: ThemeId}) {
	if (themeId === "system") {
		const light = PREVIEW_COLORS.vanilla;
		const dark = PREVIEW_COLORS.default;
		return (
			<div className="mt-3 flex h-12 w-full overflow-hidden rounded border border-app-line/50">
				<div className="flex flex-1" style={{backgroundColor: light.bg}}>
					<div
						className="w-6 border-r"
						style={{
							backgroundColor: light.sidebar,
							borderColor: light.accent + "30",
						}}
					/>
					<div className="flex flex-1 flex-col gap-1 p-1.5">
						<div
							className="h-1.5 w-10 rounded-sm"
							style={{backgroundColor: light.accent}}
						/>
						<div
							className="h-1 w-12 rounded-sm opacity-30"
							style={{backgroundColor: light.accent}}
						/>
					</div>
				</div>
				<div className="flex flex-1" style={{backgroundColor: dark.bg}}>
					<div
						className="w-6 border-r"
						style={{
							backgroundColor: dark.sidebar,
							borderColor: dark.accent + "30",
						}}
					/>
					<div className="flex flex-1 flex-col gap-1 p-1.5">
						<div
							className="h-1.5 w-10 rounded-sm"
							style={{backgroundColor: dark.accent}}
						/>
						<div
							className="h-1 w-12 rounded-sm opacity-30"
							style={{backgroundColor: dark.accent}}
						/>
					</div>
				</div>
			</div>
		);
	}

	const c = PREVIEW_COLORS[themeId];

	return (
		<div
			className="mt-3 flex h-12 w-full overflow-hidden rounded border border-app-line/50"
			style={{backgroundColor: c.bg}}
		>
			<div
				className="w-8 border-r"
				style={{backgroundColor: c.sidebar, borderColor: c.accent + "30"}}
			/>
			<div className="flex flex-1 flex-col gap-1 p-1.5">
				<div
					className="h-1.5 w-12 rounded-sm"
					style={{backgroundColor: c.accent}}
				/>
				<div
					className="h-1 w-16 rounded-sm opacity-30"
					style={{backgroundColor: c.accent}}
				/>
				<div
					className="h-1 w-10 rounded-sm opacity-20"
					style={{backgroundColor: c.accent}}
				/>
			</div>
		</div>
	);
}
