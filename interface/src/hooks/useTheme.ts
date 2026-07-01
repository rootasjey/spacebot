import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@/platform";

export type ThemeId =
	| "system"
	| "default"
	| "vanilla"
	| "midnight"
	| "noir"
	| "slate"
	| "nord"
	| "mocha";

export interface ThemeOption {
	id: ThemeId;
	name: string;
	description: string;
	className: string;
}

export const THEMES: ThemeOption[] = [
	{
		id: "system",
		name: "System",
		description: "Follows your system appearance",
		className: "",
	},
	{
		id: "default",
		name: "Default",
		description: "Dark theme with blue accent",
		className: "",
	},
	{
		id: "vanilla",
		name: "Vanilla",
		description: "Light theme",
		className: "vanilla-theme",
	},
	{
		id: "midnight",
		name: "Midnight",
		description: "Deep blue dark theme",
		className: "midnight-theme",
	},
	{
		id: "noir",
		name: "Noir",
		description: "Pure black and white theme",
		className: "noir-theme",
	},
	{
		id: "slate",
		name: "Slate",
		description: "Cool gray dark theme",
		className: "slate-theme",
	},
	{
		id: "nord",
		name: "Nord",
		description: "Arctic blue-gray theme",
		className: "nord-theme",
	},
	{
		id: "mocha",
		name: "Mocha",
		description: "Warm brown dark theme",
		className: "mocha-theme",
	},
];

export const LIGHT_THEMES: ThemeId[] = ["vanilla"];
export const DARK_THEMES: ThemeId[] = [
	"default",
	"midnight",
	"noir",
	"slate",
	"nord",
	"mocha",
];

const STORAGE_KEY = "spacebot-theme";
const STORAGE_KEY_SYSTEM_LIGHT = "spacebot-theme-system-light";
const STORAGE_KEY_SYSTEM_DARK = "spacebot-theme-system-dark";

function getStoredString(key: string, fallback: string): string {
	if (typeof window === "undefined") return fallback;
	try {
		return localStorage.getItem(key) ?? fallback;
	} catch {
		return fallback;
	}
}

function getInitialTheme(): ThemeId {
	const stored = getStoredString(STORAGE_KEY, "default");
	if (stored && THEMES.some((t) => t.id === stored)) {
		return stored as ThemeId;
	}
	return "default";
}

function getSystemLightTheme(): ThemeId {
	const stored = getStoredString(STORAGE_KEY_SYSTEM_LIGHT, "vanilla");
	if ((LIGHT_THEMES as readonly ThemeId[]).includes(stored as ThemeId)) {
		return stored as ThemeId;
	}
	return "vanilla";
}

function getSystemDarkTheme(): ThemeId {
	const stored = getStoredString(STORAGE_KEY_SYSTEM_DARK, "default");
	if ((DARK_THEMES as readonly ThemeId[]).includes(stored as ThemeId)) {
		return stored as ThemeId;
	}
	return "default";
}

/**
 * Called when the user selects "System" — queries the OS directly
 * (bypassing any NSApp.appearance lock) to get the real system
 * appearance, then applies the correct sub-theme and unlocks the
 * native appearance so prefers-color-scheme stays in sync going forward.
 */
async function applySystemTheme(systemLight: ThemeId, systemDark: ThemeId) {
	const dark = await invoke<number>("get_system_appearance");
	const isDark = dark === 1;
	const resolved = isDark ? systemDark : systemLight;
	applyThemeClass(resolved);
	invoke("set_native_theme", { themeType: -1 });
}

/**
 * Resolve a theme selection to a concrete ThemeId.
 * "system" is resolved to the appropriate light/dark sub-theme.
 * Any other ThemeId is returned as-is.
 */
export function resolveThemeId(
	themeId: ThemeId,
	systemLight: ThemeId,
	systemDark: ThemeId,
): ThemeId {
	if (themeId !== "system") return themeId;
	if (typeof window === "undefined") return systemDark;
	return window.matchMedia("(prefers-color-scheme: dark)").matches
		? systemDark
		: systemLight;
}

function applyThemeClass(actualThemeId: ThemeId) {
	if (actualThemeId === "system") return;

	const theme = THEMES.find((t) => t.id === actualThemeId);
	const root = document.documentElement;

	THEMES.forEach((t) => {
		if (t.className) {
			root.classList.remove(t.className);
		}
	});

	if (theme?.className) {
		root.classList.add(theme.className);
	}
}

function resolveAndApplyNow(themeId: ThemeId) {
	if (themeId !== "system") {
		applyThemeClass(themeId);
		return;
	}
	const light = getSystemLightTheme();
	const dark = getSystemDarkTheme();
	const resolved = window.matchMedia("(prefers-color-scheme: dark)").matches
		? dark
		: light;
	applyThemeClass(resolved);
}

async function syncNativeTheme(themeId: ThemeId, resolved: ThemeId) {
	if (themeId === "system") {
		await invoke("set_native_theme", { themeType: -1 });
	} else {
		const isLight = resolved === "vanilla";
		await invoke("set_native_theme", { themeType: isLight ? 0 : 1 });
	}
}

export function useTheme() {
	const [theme, setThemeState] = useState<ThemeId>(getInitialTheme);
	const [systemLight, setSystemLightState] =
		useState<ThemeId>(getSystemLightTheme);
	const [systemDark, setSystemDarkState] =
		useState<ThemeId>(getSystemDarkTheme);

	const themeRef = useRef(theme);
	const systemLightRef = useRef(systemLight);
	const systemDarkRef = useRef(systemDark);
	themeRef.current = theme;
	systemLightRef.current = systemLight;
	systemDarkRef.current = systemDark;

	// Apply theme + sync native whenever state changes
	useEffect(() => {
		const resolved = resolveThemeId(theme, systemLight, systemDark);
		applyThemeClass(resolved);
		syncNativeTheme(theme, resolved);
	}, [theme, systemLight, systemDark]);

	// Listen for system color scheme changes (only while in system mode)
	useEffect(() => {
		if (theme !== "system") return;
		const mq = window.matchMedia("(prefers-color-scheme: dark)");
		const handler = () => {
			const resolved = mq.matches
				? systemDarkRef.current
				: systemLightRef.current;
			applyThemeClass(resolved);
		};
		mq.addEventListener("change", handler);
		return () => mq.removeEventListener("change", handler);
	}, [theme]);

	const setTheme = useCallback((newTheme: ThemeId) => {
		setThemeState(newTheme);
		localStorage.setItem(STORAGE_KEY, newTheme);
		if (newTheme === "system") {
			// Query the OS directly to bypass any NSApp lock
			applySystemTheme(
				systemLightRef.current,
				systemDarkRef.current,
			);
		} else {
			resolveAndApplyNow(newTheme);
			const isLight = newTheme === "vanilla";
			invoke("set_native_theme", { themeType: isLight ? 0 : 1 });
		}
	}, []);

	const setSystemLight = useCallback((newTheme: ThemeId) => {
		setSystemLightState(newTheme);
		localStorage.setItem(STORAGE_KEY_SYSTEM_LIGHT, newTheme);
		if (themeRef.current === "system") {
			const dark = getSystemDarkTheme();
			const resolved = window.matchMedia("(prefers-color-scheme: dark)").matches
				? dark
				: newTheme;
			applyThemeClass(resolved);
		}
	}, []);

	const setSystemDark = useCallback((newTheme: ThemeId) => {
		setSystemDarkState(newTheme);
		localStorage.setItem(STORAGE_KEY_SYSTEM_DARK, newTheme);
		if (themeRef.current === "system") {
			const light = getSystemLightTheme();
			const resolved = window.matchMedia("(prefers-color-scheme: dark)").matches
				? newTheme
				: light;
			applyThemeClass(resolved);
		}
	}, []);

	return {
		theme,
		setTheme,
		systemLight,
		setSystemLight,
		systemDark,
		setSystemDark,
		themes: THEMES,
	};
}

// Initialize theme on page load (before React hydrates) — CSS only, no native sync
if (typeof window !== "undefined") {
	const initialTheme = getInitialTheme();
	resolveAndApplyNow(initialTheme);
}
