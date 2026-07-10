<script lang="ts">
	import { hub, send } from '$lib/hub.svelte';
	import { kindName, quality, supportsBrightness, type Entity } from '$lib/types';

	let { entity }: { entity: Entity } = $props();

	const q = $derived(quality(entity, hub.now));
	const kind = $derived(kindName(entity.kind));
	const device = $derived(hub.devices[entity.device_id]?.name ?? entity.device_id);
	const isPending = $derived(entity.id in hub.pending);
	const isOn = $derived(entity.state?.value === true);
	const brightness = $derived(Number(entity.attributes['brightness'] ?? 255));

	const QUALITY_LABEL: Record<string, string | null> = {
		live: null,
		retained: 'last known',
		stale: 'stale',
		unavailable: 'offline'
	};

	function fmt(v: unknown): string {
		if (typeof v === 'number') return Number.isInteger(v) ? String(v) : v.toFixed(1);
		if (typeof v === 'boolean') {
			if (entity.device_class === 'door' || entity.device_class === 'window')
				return v ? 'open' : 'closed';
			return v ? 'on' : 'off';
		}
		return String(v ?? '—');
	}

	function toggle() {
		send(entity.id, { command: isOn ? 'turn_off' : 'turn_on' });
	}
</script>

<article class="card {q}" class:pending={isPending}>
	<header>
		<h3>{entity.name}</h3>
		<span class="lamp" title={q}></span>
	</header>

	{#if kind === 'switch' || kind === 'light'}
		<button class="toggle" class:on={isOn} onclick={toggle} disabled={q === 'unavailable'}>
			<span class="knob"></span>
			<span class="toggle-label">{fmt(entity.state?.value)}</span>
		</button>
		{#if kind === 'light' && supportsBrightness(entity)}
			<input
				class="dimmer"
				type="range"
				min="1"
				max="255"
				value={brightness}
				disabled={q === 'unavailable'}
				aria-label="{entity.name} brightness"
				onchange={(ev) =>
					send(entity.id, { command: 'set_brightness', brightness: Number(ev.currentTarget.value) })}
			/>
		{/if}
	{:else if kind === 'cover'}
		<div class="value moving">{fmt(entity.state?.value)}</div>
		<div class="cover-controls">
			<button onclick={() => send(entity.id, { command: 'open' })} disabled={q === 'unavailable'}>Open</button>
			<button onclick={() => send(entity.id, { command: 'stop' })} disabled={q === 'unavailable'}>Stop</button>
			<button onclick={() => send(entity.id, { command: 'close' })} disabled={q === 'unavailable'}>Close</button>
		</div>
	{:else}
		<div class="value">
			{fmt(entity.state?.value)}{#if entity.unit}<span class="unit">{entity.unit}</span>{/if}
		</div>
	{/if}

	<footer>
		<span class="device">{device}</span>
		{#if QUALITY_LABEL[q]}<span class="qlabel">{QUALITY_LABEL[q]}</span>{/if}
	</footer>
</article>

<style>
	.card {
		background: var(--surface);
		border: 1px solid var(--line);
		border-radius: var(--radius);
		padding: 0.9rem 1rem 0.75rem;
		display: flex;
		flex-direction: column;
		gap: 0.55rem;
		min-height: 7.2rem;
		transition: opacity 150ms;
	}
	.card.unavailable {
		opacity: 0.45;
	}
	.card.pending {
		border-color: var(--amber);
	}

	header {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: 0.5rem;
	}
	h3 {
		margin: 0;
		font-size: 0.85rem;
		font-weight: 500;
		color: var(--dim);
		letter-spacing: 0.01em;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.lamp {
		flex: none;
		width: 8px;
		height: 8px;
		border-radius: 50%;
		align-self: center;
	}
	.live .lamp {
		background: var(--lamp-live);
		box-shadow: 0 0 6px var(--lamp-live);
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
	.pending .lamp {
		animation: blink 0.8s infinite;
	}
	@keyframes blink {
		50% {
			opacity: 0.2;
		}
	}

	.value {
		font-family: var(--font-data);
		font-size: 1.7rem;
		font-weight: 500;
		letter-spacing: -0.02em;
		flex: 1;
	}
	.retained .value,
	.stale .value {
		color: var(--dim);
	}
	.unit {
		font-size: 0.85rem;
		color: var(--dim);
		margin-left: 0.35rem;
	}
	.moving {
		font-size: 1.15rem;
		text-transform: capitalize;
	}

	.toggle {
		display: flex;
		align-items: center;
		gap: 0.6rem;
		background: var(--surface-2);
		border: 1px solid var(--line);
		border-radius: 999px;
		padding: 0.3rem 0.9rem 0.3rem 0.3rem;
		cursor: pointer;
		width: fit-content;
	}
	.toggle:disabled {
		cursor: default;
	}
	.knob {
		width: 1.5rem;
		height: 1.5rem;
		border-radius: 50%;
		background: var(--faint);
		transition: background 150ms;
	}
	.toggle.on .knob {
		background: var(--amber);
		box-shadow: 0 0 10px var(--amber-soft), 0 0 4px var(--amber);
	}
	.toggle-label {
		font-family: var(--font-data);
		font-size: 0.85rem;
		text-transform: uppercase;
		color: var(--dim);
	}
	.toggle.on .toggle-label {
		color: var(--text);
	}

	.dimmer {
		width: 100%;
		accent-color: var(--amber);
	}

	.cover-controls {
		display: flex;
		gap: 0.4rem;
	}
	.cover-controls button {
		flex: 1;
		background: var(--surface-2);
		border: 1px solid var(--line);
		border-radius: 6px;
		padding: 0.4rem 0;
		font-size: 0.8rem;
		cursor: pointer;
	}
	.cover-controls button:hover:not(:disabled) {
		border-color: var(--amber);
	}

	footer {
		display: flex;
		justify-content: space-between;
		font-size: 0.72rem;
		color: var(--faint);
	}
	.qlabel {
		color: var(--dim);
		font-style: italic;
	}
</style>
