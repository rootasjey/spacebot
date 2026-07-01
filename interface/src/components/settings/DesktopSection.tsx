import { useState, useEffect, useRef, useCallback } from "react";
import { IS_DESKTOP, invoke as platformInvoke } from "@/platform";
import { Switch } from "@spacedrive/primitives";

interface ShortcutConfig {
  enabled: boolean;
  key: string;
}

function displayKey(key: string): string {
  return key
    .replace("Alt", "Option")
    .replace("Cmd", "⌘")
    .replace("Shift", "⇧")
    .replace("Ctrl", "⌃");
}

function KeyRecorder({
  value,
  onChange,
  disabled,
}: {
  value: string;
  onChange: (v: string) => void;
  disabled?: boolean;
}) {
  const [recording, setRecording] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (!recording) return;
      e.preventDefault();
      e.stopPropagation();

      if (e.key === "Escape") {
        setRecording(false);
        return;
      }

      const parts: string[] = [];
      if (e.metaKey) parts.push("Cmd");
      if (e.ctrlKey) parts.push("Ctrl");
      if (e.altKey) parts.push("Alt");
      if (e.shiftKey) parts.push("Shift");

      const keyMap: Record<string, string> = {
        " ": "Space",
        Escape: "Escape",
        Enter: "Enter",
        Tab: "Tab",
        Backspace: "Backspace",
        Delete: "Delete",
        ArrowUp: "ArrowUp",
        ArrowDown: "ArrowDown",
        ArrowLeft: "ArrowLeft",
        ArrowRight: "ArrowRight",
        Home: "Home",
        End: "End",
        PageUp: "PageUp",
        PageDown: "PageDown",
      };

      let mainKey = keyMap[e.key] ?? e.key;
      if (mainKey === "Meta" || mainKey === "Control" || mainKey === "Alt" || mainKey === "Shift") {
        return;
      }

      // Single-letter and single-digit keys: uppercase
      if (mainKey.length === 1) {
        mainKey = mainKey.toUpperCase();
      }

      // Ignore IME/Composition keys
      if (e.isComposing || e.keyCode === 229) return;

      parts.push(mainKey);
      const combo = parts.join("+");
      onChange(combo);
      setRecording(false);
    },
    [recording, onChange],
  );

  useEffect(() => {
    if (!recording) return;
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [recording, handleKeyDown]);

  return (
    <div
      ref={ref}
      tabIndex={disabled ? -1 : 0}
      role="button"
      onClick={() => {
        if (!disabled) setRecording(true);
      }}
      onBlur={() => setRecording(false)}
      className={`flex cursor-pointer items-center rounded-md border px-3 py-1.5 text-sm outline-none transition-colors ${
        recording
          ? "border-accent bg-accent/10 text-accent"
          : "border-app-line bg-app-dark-box text-ink hover:border-app-selected"
      } ${disabled ? "cursor-not-allowed opacity-50" : ""}`}
    >
      {recording ? "Press shortcut\u2026" : displayKey(value) || "Click to set"}
    </div>
  );
}

export function DesktopSection() {
  const [toggle, setToggle] = useState<ShortcutConfig>({
    enabled: true,
    key: "Alt+Space",
  });
  const [recording, setRecording] = useState<ShortcutConfig>({
    enabled: true,
    key: "Alt+Shift+Space",
  });
  const [loading, setLoading] = useState(true);
  const [message, setMessage] = useState<{
    text: string;
    type: "success" | "error";
  } | null>(null);

  useEffect(() => {
    if (!IS_DESKTOP) {
      setLoading(false);
      return;
    }
    Promise.all([
      platformInvoke<boolean>("get_toggle_shortcut_state"),
      platformInvoke<string>("get_toggle_shortcut_key"),
      platformInvoke<boolean>("get_recording_shortcut_state"),
      platformInvoke<string>("get_recording_shortcut_key"),
    ])
      .then(([toggleEnabled, toggleKey, recEnabled, recKey]) => {
        setToggle({
          enabled: toggleEnabled ?? true,
          key: toggleKey ?? "Alt+Space",
        });
        setRecording({
          enabled: recEnabled ?? true,
          key: recKey ?? "Alt+Shift+Space",
        });
        setLoading(false);
      })
      .catch(() => setLoading(false));
  }, []);

  const saveToggle = async (cfg: ShortcutConfig) => {
    setToggle(cfg);
    setMessage(null);
    try {
      await platformInvoke("set_toggle_shortcut", {
        enabled: cfg.enabled,
        key: cfg.key,
      });
      setMessage({ text: "Toggle shortcut saved.", type: "success" });
    } catch {
      setMessage({
        text: "Failed to save toggle shortcut.",
        type: "error",
      });
    }
  };

  const saveRecording = async (cfg: ShortcutConfig) => {
    setRecording(cfg);
    setMessage(null);
    try {
      await platformInvoke("set_recording_shortcut", {
        enabled: cfg.enabled,
        key: cfg.key,
      });
      setMessage({ text: "Recording shortcut saved.", type: "success" });
    } catch {
      setMessage({
        text: "Failed to save recording shortcut.",
        type: "error",
      });
    }
  };

  if (!IS_DESKTOP) {
    return (
      <div className="mx-auto max-w-2xl px-6 py-6">
        <p className="text-sm text-ink-dull">
          Desktop settings are only available in the desktop app.
        </p>
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-2xl px-6 py-6">
      <div className="mb-6">
        <h2 className="font-plex text-sm font-semibold text-ink">Desktop</h2>
        <p className="mt-1 text-sm text-ink-dull">
          Configure keyboard shortcuts for the voice overlay window.
        </p>
      </div>

      {loading ? (
        <div className="flex items-center gap-2 text-ink-dull">
          <div className="h-2 w-2 animate-pulse rounded-full bg-accent" />
          Loading...
        </div>
      ) : (
        <div className="flex flex-col gap-4">
          {/* Toggle overlay shortcut */}
          <div className="rounded-lg border border-app-line bg-app-box p-4">
            <div className="flex items-center justify-between">
              <div>
                <span className="text-sm font-medium text-ink">
                  Show / Hide Overlay
                </span>
                <p className="mt-0.5 text-sm text-ink-dull">
                  Toggle the voice overlay window.
                </p>
              </div>
              <Switch
                size="md"
                checked={toggle.enabled}
                onCheckedChange={(enabled) =>
                  saveToggle({ ...toggle, enabled })
                }
              />
            </div>
            <div className="mt-3 flex items-center gap-3">
              <span className="text-xs text-ink-dull">Shortcut</span>
              <KeyRecorder
                value={toggle.key}
                onChange={(key) => saveToggle({ ...toggle, key })}
                disabled={!toggle.enabled}
              />
            </div>
          </div>

          {/* Recording shortcut */}
          <div className="rounded-lg border border-app-line bg-app-box p-4">
            <div className="flex items-center justify-between">
              <div>
                <span className="text-sm font-medium text-ink">
                  Start / Stop Recording
                </span>
                <p className="mt-0.5 text-sm text-ink-dull">
                  Press and hold to record, release to stop.
                </p>
              </div>
              <Switch
                size="md"
                checked={recording.enabled}
                onCheckedChange={(enabled) =>
                  saveRecording({ ...recording, enabled })
                }
              />
            </div>
            <div className="mt-3 flex items-center gap-3">
              <span className="text-xs text-ink-dull">Shortcut</span>
              <KeyRecorder
                value={recording.key}
                onChange={(key) => saveRecording({ ...recording, key })}
                disabled={!recording.enabled}
              />
            </div>
          </div>
        </div>
      )}

      {message && (
        <div
          className={`mt-4 rounded-md border px-3 py-2 text-sm ${
            message.type === "success"
              ? "border-green-500/20 bg-green-500/10 text-green-400"
              : "border-red-500/20 bg-red-500/10 text-red-400"
          }`}
        >
          {message.text}
        </div>
      )}
    </div>
  );
}
