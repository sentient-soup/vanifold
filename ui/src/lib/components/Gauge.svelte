<script lang="ts">
	let {
		value,
		label,
		unit = '%',
		tone = 'ok',
		size = 'large'
	}: {
		value: number | null;
		label: string;
		unit?: string;
		tone?: 'ok' | 'warn' | 'crit';
		size?: 'large' | 'small';
	} = $props();

	// 270-degree arc, gap at the bottom, drawn with pathLength=100 dash math.
	const SWEEP = 75; // of pathLength 100
	const arc = $derived(value === null ? 0 : Math.max(0, Math.min(100, value)) * (SWEEP / 100));
</script>

<div class="gauge {size} {tone}">
	<svg viewBox="0 0 120 120">
		<circle class="track" cx="60" cy="60" r="52" pathLength="100" stroke-dasharray="{SWEEP} {100 - SWEEP}" />
		{#if value !== null}
			<circle class="fill" cx="60" cy="60" r="52" pathLength="100" stroke-dasharray="{arc} {100 - arc}" />
		{/if}
	</svg>
	<div class="center">
		<div class="reading">
			<span class="num">{value === null ? '—' : Math.round(value)}</span>{#if value !== null}<span class="unit">{unit}</span>{/if}
		</div>
		<div class="label">{label}</div>
	</div>
</div>

<style>
	.gauge {
		position: relative;
		width: 100%;
		max-width: 220px;
		aspect-ratio: 1;
	}
	.gauge.small {
		max-width: 128px;
	}
	svg {
		width: 100%;
		height: 100%;
		transform: rotate(135deg);
	}
	circle {
		fill: none;
		stroke-width: 7;
		stroke-linecap: round;
	}
	.track {
		stroke: var(--line);
	}
	.fill {
		stroke: var(--amber);
		transition: stroke-dasharray 600ms ease;
	}
	.warn .fill {
		stroke: var(--lamp-stale);
	}
	.crit .fill {
		stroke: var(--lamp-unavailable);
	}

	.center {
		position: absolute;
		inset: 0;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 0.1rem;
	}
	.reading {
		display: flex;
		align-items: baseline;
	}
	.num {
		font-family: var(--font-display);
		font-weight: 600;
		font-size: clamp(2rem, 18cqw, 3.4rem);
		line-height: 1;
		letter-spacing: -0.01em;
	}
	.gauge.small .num {
		font-size: 1.9rem;
	}
	.unit {
		font-family: var(--font-data);
		font-size: 0.9rem;
		color: var(--dim);
		margin-left: 0.15rem;
	}
	.gauge.small .unit {
		font-size: 0.7rem;
	}
	.label {
		font-size: 0.7rem;
		text-transform: uppercase;
		letter-spacing: 0.12em;
		color: var(--dim);
	}
	.crit .num {
		color: var(--lamp-unavailable);
	}
</style>
