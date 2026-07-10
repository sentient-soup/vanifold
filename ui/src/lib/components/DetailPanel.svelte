<script lang="ts">
	import { fetchHistory, hub, patchEntity } from '$lib/hub.svelte';
	import { kindName, quality, type Entity } from '$lib/types';

	const entity = $derived(hub.selected ? hub.entities[hub.selected] : null);
	const q = $derived(entity ? quality(entity, hub.now) : 'stale');
	const device = $derived(entity ? hub.devices[entity.device_id] : null);

	const SUBSYSTEMS = ['', 'power', 'climate', 'plumbing', 'vehicle', 'computer', 'misc'];
	const CRITICALITIES = ['info', 'comfort', 'safety'];

	type Point = { ts: number; vmin: number; vmax: number; vavg: number };
	let points = $state<Point[] | null>(null);

	$effect(() => {
		points = null;
		const id = entity?.id;
		if (!id) return;
		fetchHistory(id).then((p) => {
			// Drop out-of-date responses if the selection changed meanwhile.
			if (hub.selected === id) points = p;
		});
	});

	const chart = $derived.by(() => {
		if (!points || points.length < 2) return null;
		const W = 560;
		const H = 140;
		const PAD = 4;
		const t0 = points[0].ts;
		const t1 = points[points.length - 1].ts;
		let lo = Math.min(...points.map((p) => p.vmin));
		let hi = Math.max(...points.map((p) => p.vmax));
		if (hi - lo < 1e-9) {
			hi += 1;
			lo -= 1;
		}
		const x = (ts: number) => PAD + ((ts - t0) / (t1 - t0)) * (W - 2 * PAD);
		const y = (v: number) => H - PAD - ((v - lo) / (hi - lo)) * (H - 2 * PAD);
		const line = points.map((p, i) => `${i ? 'L' : 'M'}${x(p.ts).toFixed(1)},${y(p.vavg).toFixed(1)}`).join('');
		const band =
			points.map((p, i) => `${i ? 'L' : 'M'}${x(p.ts).toFixed(1)},${y(p.vmax).toFixed(1)}`).join('') +
			[...points].reverse().map((p) => `L${x(p.ts).toFixed(1)},${y(p.vmin).toFixed(1)}`).join('') +
			'Z';
		return { line, band, lo, hi, W, H };
	});

	function close() {
		hub.selected = null;
	}

	function rename(e: Entity, value: string) {
		if (value.trim() && value.trim() !== e.name) patchEntity(e.id, { name: value.trim() });
	}
</script>

{#if entity}
	<div class="scrim" onclick={close} role="presentation"></div>
	<aside class="drawer" aria-label="{entity.name} details">
		<header>
			<input
				class="name"
				value={entity.name}
				aria-label="Rename entity"
				onchange={(ev) => entity && rename(entity, ev.currentTarget.value)}
			/>
			<button class="close" onclick={close} aria-label="Close details">×</button>
		</header>

		<div class="reading {q}">
			<span class="big">
				{entity.state?.value ?? '—'}{#if entity.unit}<span class="unit">{entity.unit}</span>{/if}
			</span>
			<span class="q">{q}</span>
		</div>

		{#if points === null}
			<div class="chart-slot dim">loading history…</div>
		{:else if chart}
			<div class="chart-slot">
				<svg viewBox="0 0 {chart.W} {chart.H}" preserveAspectRatio="none" role="img" aria-label="12 hour history">
					<path d={chart.band} class="band" />
					<path d={chart.line} class="line" />
				</svg>
				<div class="scale"><span>{chart.lo.toFixed(1)}</span><span>12h</span><span>{chart.hi.toFixed(1)}</span></div>
			</div>
		{:else}
			<div class="chart-slot dim">no numeric history yet</div>
		{/if}

		<div class="meta">
			<label>
				subsystem
				<select
					value={entity.subsystem ?? ''}
					onchange={(ev) => entity && patchEntity(entity.id, { subsystem: ev.currentTarget.value })}
				>
					{#each SUBSYSTEMS as s (s)}
						<option value={s}>{s === '' ? '(unassigned)' : s}</option>
					{/each}
				</select>
			</label>
			<label>
				criticality
				<select
					value={entity.criticality}
					onchange={(ev) => entity && patchEntity(entity.id, { criticality: ev.currentTarget.value })}
				>
					{#each CRITICALITIES as c (c)}
						<option value={c}>{c}</option>
					{/each}
				</select>
			</label>
		</div>

		<dl class="facts">
			<dt>kind</dt>
			<dd>{kindName(entity.kind)}{entity.device_class ? ` · ${entity.device_class}` : ''}</dd>
			<dt>device</dt>
			<dd>{device?.name ?? entity.device_id}{device?.model ? ` (${device.manufacturer ?? ''} ${device.model})` : ''}</dd>
			<dt>id</dt>
			<dd class="mono">{entity.id}</dd>
			{#if entity.state}
				<dt>updated</dt>
				<dd>{new Date(entity.state.updated_at).toLocaleTimeString()}{entity.state.retained ? ' (retained)' : ''}</dd>
			{/if}
		</dl>
	</aside>
{/if}

<style>
	.scrim {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.45);
		z-index: 10;
	}
	.drawer {
		position: fixed;
		top: 0;
		right: 0;
		bottom: 0;
		width: min(420px, 100vw);
		background: var(--bg);
		border-left: 1px solid var(--line);
		padding: 1.1rem 1.2rem;
		overflow-y: auto;
		z-index: 11;
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	header {
		display: flex;
		gap: 0.6rem;
		align-items: center;
	}
	.name {
		flex: 1;
		background: transparent;
		border: 1px solid transparent;
		border-radius: 6px;
		color: var(--text);
		font-family: var(--font-ui);
		font-size: 1.15rem;
		font-weight: 600;
		padding: 0.25rem 0.4rem;
	}
	.name:hover,
	.name:focus {
		border-color: var(--line);
		background: var(--surface);
	}
	.close {
		background: none;
		border: none;
		font-size: 1.5rem;
		color: var(--dim);
		cursor: pointer;
		line-height: 1;
	}

	.reading {
		display: flex;
		align-items: baseline;
		gap: 0.8rem;
	}
	.big {
		font-family: var(--font-data);
		font-size: 2.4rem;
		font-weight: 500;
	}
	.unit {
		font-size: 1rem;
		color: var(--dim);
		margin-left: 0.4rem;
	}
	.reading.stale .big,
	.reading.retained .big {
		color: var(--dim);
	}
	.reading.unavailable .big {
		color: var(--faint);
	}
	.q {
		font-family: var(--font-data);
		font-size: 0.75rem;
		text-transform: uppercase;
		letter-spacing: 0.08em;
		color: var(--dim);
	}

	.chart-slot {
		background: var(--surface);
		border: 1px solid var(--line);
		border-radius: var(--radius);
		padding: 0.6rem;
	}
	.chart-slot.dim {
		color: var(--faint);
		font-size: 0.85rem;
		text-align: center;
		padding: 2.4rem 0;
	}
	svg {
		display: block;
		width: 100%;
		height: 140px;
	}
	.band {
		fill: var(--amber-soft);
		stroke: none;
	}
	.line {
		fill: none;
		stroke: var(--amber);
		stroke-width: 1.6;
	}
	.scale {
		display: flex;
		justify-content: space-between;
		font-family: var(--font-data);
		font-size: 0.7rem;
		color: var(--faint);
		padding-top: 0.3rem;
	}

	.meta {
		display: flex;
		gap: 0.8rem;
	}
	label {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
		font-size: 0.72rem;
		text-transform: uppercase;
		letter-spacing: 0.08em;
		color: var(--dim);
	}
	select {
		background: var(--surface);
		border: 1px solid var(--line);
		border-radius: 6px;
		color: var(--text);
		font-family: var(--font-ui);
		font-size: 0.9rem;
		padding: 0.4rem;
	}

	.facts {
		margin: 0;
		display: grid;
		grid-template-columns: auto 1fr;
		gap: 0.35rem 1rem;
		font-size: 0.85rem;
	}
	dt {
		color: var(--faint);
	}
	dd {
		margin: 0;
		color: var(--dim);
	}
	.mono {
		font-family: var(--font-data);
		font-size: 0.8rem;
	}
</style>
