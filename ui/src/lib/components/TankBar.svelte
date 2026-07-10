<script lang="ts">
	let {
		label,
		value,
		hue = 'water',
		tone = 'ok'
	}: {
		label: string;
		value: number | null;
		hue?: 'water' | 'grey' | 'amber';
		tone?: 'ok' | 'warn' | 'crit';
	} = $props();

	const width = $derived(value === null ? 0 : Math.max(0, Math.min(100, value)));
</script>

<div class="tank {tone}">
	<div class="row">
		<span class="label">{label}</span>
		<span class="value">{value === null ? '—' : `${Math.round(value)}%`}</span>
	</div>
	<div class="track" role="meter" aria-valuenow={value ?? undefined} aria-valuemin="0" aria-valuemax="100" aria-label={label}>
		<div class="fill {hue}" style:width="{width}%"></div>
	</div>
</div>

<style>
	.tank {
		display: flex;
		flex-direction: column;
		gap: 0.35rem;
	}
	.row {
		display: flex;
		justify-content: space-between;
		align-items: baseline;
	}
	.label {
		font-size: 0.72rem;
		text-transform: uppercase;
		letter-spacing: 0.1em;
		color: var(--dim);
	}
	.value {
		font-family: var(--font-display);
		font-weight: 600;
		font-size: 1.35rem;
		line-height: 1;
	}
	.warn .value {
		color: var(--lamp-stale);
	}
	.crit .value {
		color: var(--lamp-unavailable);
	}
	.track {
		height: 10px;
		border-radius: 5px;
		background: var(--surface-2);
		border: 1px solid var(--line);
		overflow: hidden;
	}
	.fill {
		height: 100%;
		border-radius: 5px;
		transition: width 600ms ease;
	}
	.fill.water {
		background: var(--water);
	}
	.fill.grey {
		background: var(--faint);
	}
	.fill.amber {
		background: var(--amber);
	}
	.warn .fill,
	.crit .fill {
		background: var(--lamp-stale);
	}
	.crit .fill {
		background: var(--lamp-unavailable);
	}
</style>
