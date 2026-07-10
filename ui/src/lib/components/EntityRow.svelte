<script lang="ts">
	import { hub } from '$lib/hub.svelte';
	import { quality, type Entity } from '$lib/types';

	let { entity }: { entity: Entity } = $props();

	const q = $derived(quality(entity, hub.now));

	function fmtValue(): string {
		const v = entity.state?.value;
		if (v === undefined || v === null) return '—';
		if (typeof v === 'number') return Number.isInteger(v) ? String(v) : v.toFixed(1);
		if (typeof v === 'boolean') {
			if (entity.device_class === 'door' || entity.device_class === 'window')
				return v ? 'open' : 'closed';
			return v ? 'on' : 'off';
		}
		return String(v);
	}
</script>

<button class="row {q}" onclick={() => (hub.selected = entity.id)}>
	<span class="lamp"></span>
	<span class="name">{entity.name}</span>
	<span class="value">{fmtValue()}{#if entity.unit}<span class="unit">{entity.unit}</span>{/if}</span>
	<span class="chev">›</span>
</button>

<style>
	.row {
		display: flex;
		align-items: center;
		gap: 0.7rem;
		width: 100%;
		min-height: 48px;
		background: none;
		border: none;
		border-bottom: 1px solid var(--line);
		padding: 0.35rem 0.3rem;
		cursor: pointer;
		color: var(--text);
		font-family: var(--font-ui);
		font-size: 0.92rem;
		text-align: left;
	}
	.row:hover {
		background: var(--surface);
	}
	.row.unavailable {
		opacity: 0.45;
	}

	.lamp {
		flex: none;
		width: 7px;
		height: 7px;
		border-radius: 50%;
	}
	.live .lamp {
		background: var(--lamp-live);
		box-shadow: 0 0 5px var(--lamp-live);
	}
	.retained .lamp {
		background: transparent;
		border: 1.5px solid var(--lamp-retained);
	}
	.stale .lamp {
		background: var(--lamp-stale);
	}
	.unavailable .lamp {
		background: var(--lamp-unavailable);
	}

	.name {
		flex: 1;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.value {
		font-family: var(--font-data);
		font-size: 0.95rem;
	}
	.stale .value,
	.retained .value {
		color: var(--dim);
	}
	.unit {
		color: var(--faint);
		font-size: 0.75rem;
		margin-left: 0.25rem;
	}
	.chev {
		color: var(--faint);
	}
</style>
