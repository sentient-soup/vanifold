<script lang="ts">
	let { name, size = 22 }: { name: string; size?: number } = $props();

	// Static stroke icons, one per panel/action. Inner SVG markup is constant
	// app code, never user input.
	const ICONS: Record<string, string> = {
		power: '<path d="M13 2 L4.5 13.5 H11 L10 22 L19.5 9.5 H13 Z"/>',
		water: '<path d="M12 3 C12 3 6 10.2 6 14.6 A6 6 0 0 0 18 14.6 C18 10.2 12 3 12 3 Z"/>',
		climate:
			'<path d="M10 6 a2 2 0 0 1 4 0 v7.2 a4 4 0 1 1 -4 0 Z"/><circle cx="12" cy="17" r="1.4"/>',
		lighting:
			'<path d="M12 3 a6 6 0 0 0 -3.5 10.8 c .9 .7 1 1.5 1 2.7 h 5 c 0 -1.2 .1 -2 1 -2.7 A6 6 0 0 0 12 3 Z"/><path d="M10 20 h4"/>',
		access:
			'<path d="M4 21 h16"/><path d="M6 21 V5 a1.5 1.5 0 0 1 1.5 -1.5 h9 A1.5 1.5 0 0 1 18 5 v16"/><circle cx="15" cy="12" r="0.9"/>',
		vehicle:
			'<path d="M4 16 v-3 l2 -4.6 A2 2 0 0 1 7.9 7 h8.2 a2 2 0 0 1 1.9 1.4 L20 13 v3"/><path d="M4 13 h16"/><circle cx="7.5" cy="16.5" r="1.6"/><circle cx="16.5" cy="16.5" r="1.6"/>',
		media: '<rect x="4" y="5" width="16" height="11" rx="1.5"/><path d="M9 20 h6 M12 16 v4"/>',
		misc: '<circle cx="6" cy="12" r="1.6"/><circle cx="12" cy="12" r="1.6"/><circle cx="18" cy="12" r="1.6"/>',
		home: '<path d="M4 11 L12 4 l8 7"/><path d="M6 10 v10 h12 V10"/>',
		system:
			'<path d="M4 7 h9 M17 7 h3"/><circle cx="15" cy="7" r="2"/><path d="M4 15 h3 M11 15 h9"/><circle cx="9" cy="15" r="2"/>',
		back: '<path d="M14 6 l-6 6 6 6"/>',
		alert: '<path d="M12 4 L22 20 H2 Z"/><path d="M12 10 v4 M12 17 v.01"/>'
	};
</script>

<svg
	width={size}
	height={size}
	viewBox="0 0 24 24"
	fill="none"
	stroke="currentColor"
	stroke-width="1.7"
	stroke-linecap="round"
	stroke-linejoin="round"
	aria-hidden="true"
>
	<!-- eslint-disable-next-line svelte/no-at-html-tags -- static app constants -->
	{@html ICONS[name] ?? ICONS.misc}
</svg>
