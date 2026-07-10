<script lang="ts">
	import { page } from '$app/state';
	import { hub, href, send } from '$lib/hub.svelte';
	import {
		PANELS,
		accessSummary,
		climateHero,
		computeAlerts,
		fmt,
		isPanelKey,
		levelTone,
		lights,
		num,
		ofSubsystem,
		powerHero,
		vehicleHero,
		waterHero,
		type PanelKey
	} from '$lib/panels';
	import { kindName, type Entity } from '$lib/types';
	import Gauge from '$lib/components/Gauge.svelte';
	import TankBar from '$lib/components/TankBar.svelte';
	import ControlTile from '$lib/components/ControlTile.svelte';
	import EntityRow from '$lib/components/EntityRow.svelte';
	import Icon from '$lib/components/Icon.svelte';

	const key = $derived(isPanelKey(page.params.key ?? '') ? (page.params.key as PanelKey) : null);
	const label = $derived(PANELS.find((p) => p.key === key)?.label ?? 'Unknown');
	const list = $derived(key ? ofSubsystem(hub.entities, key) : []);
	const alerts = $derived(key ? computeAlerts(hub.entities, hub.now).filter((a) => a.panel === key) : []);

	// Per-panel heroes claim entities; the rest split into controls + sensors.
	const power = $derived(key === 'power' ? powerHero(list) : null);
	const water = $derived(key === 'water' ? waterHero(list) : null);
	const climate = $derived(key === 'climate' ? climateHero(list) : null);
	const access = $derived(key === 'access' ? accessSummary(list) : null);
	const vehicle = $derived(key === 'vehicle' ? vehicleHero(list) : null);
	const lightList = $derived(key === 'lighting' ? lights(list) : []);
	const lightsOn = $derived(lightList.filter((l) => l.state?.value === true));

	const used = $derived.by(() => {
		const u = new Set<string>();
		for (const h of [power, water, climate, vehicle]) h?.used.forEach((id) => u.add(id));
		// Access movers render as hero control tiles; openings render as rows below.
		access?.movers.forEach((e) => u.add(e.id));
		lightList.forEach((e) => u.add(e.id));
		return u;
	});

	const controls = $derived(list.filter((e) => e.command && !used.has(e.id)));
	const sensors = $derived(list.filter((e) => !e.command && !used.has(e.id)));

	const byDevice = $derived.by(() => {
		const groups = new Map<string, Entity[]>();
		for (const e of sensors) {
			if (!groups.has(e.device_id)) groups.set(e.device_id, []);
			groups.get(e.device_id)!.push(e);
		}
		return [...groups.entries()].map(([id, items]) => ({
			name: hub.devices[id]?.name ?? id,
			items
		}));
	});

	function allLightsOff() {
		for (const l of lightsOn) send(l.id, { command: 'turn_off' });
	}
</script>

<div class="panel">
	<header class="head">
		<a class="back" href={href('/')} aria-label="Back to overview"><Icon name="back" /></a>
		<Icon name={key ?? 'misc'} size={20} />
		<h1>{label}</h1>
		<div class="chips">
			{#each alerts as a (a.text)}
				<span class="chip {a.level}">{a.text}</span>
			{/each}
		</div>
	</header>

	{#if !key}
		<p class="none">This panel does not exist. <a href={href('/')}>Back to the overview.</a></p>
	{:else if list.length === 0}
		<p class="none">
			Nothing assigned to {label} yet. Entities land here automatically as devices announce
			themselves, or assign them manually from any entity's detail view.
		</p>
	{:else}
		{#if key === 'power' && power}
			<section class="hero">
				<Gauge value={num(power.soc)} label="battery" tone={levelTone(num(power.soc), { warnBelow: 20, critBelow: 10 })} />
				<div class="hero-stats">
					<div class="stat"><span class="v">{fmt(num(power.voltage), 1)}<small>V</small></span><span class="l">battery voltage</span></div>
					{#if power.solar}
						<div class="stat"><span class="v">{fmt(num(power.solar))}<small>W</small></span><span class="l">solar</span></div>
					{/if}
					<div class="stat"><span class="v">{fmt(power.wattsTotal)}<small>W</small></span><span class="l">measured circuits</span></div>
					{#if power.batteryTemp}
						<div class="stat"><span class="v">{fmt(num(power.batteryTemp), 1)}<small>°C</small></span><span class="l">battery temp</span></div>
					{/if}
				</div>
			</section>
		{:else if key === 'water' && water}
			<section class="hero stack-hero">
				<div class="tanks">
					{#each water.tanks as t (t.entity.id)}
						<TankBar
							label={t.entity.name}
							value={num(t.entity)}
							hue={t.role === 'grey' ? 'grey' : 'water'}
							tone={t.role === 'grey' ? levelTone(num(t.entity), { warnAbove: 85 }) : levelTone(num(t.entity), { warnBelow: 15 })}
						/>
					{/each}
				</div>
				{#if water.pump}
					<div class="hero-control"><ControlTile entity={water.pump} /></div>
				{/if}
			</section>
		{:else if key === 'climate' && climate}
			<section class="hero">
				<div class="stat big-stat">
					<span class="v giant">{fmt(num(climate.inside), 1)}<small>°C</small></span>
					<span class="l">inside</span>
				</div>
				{#if climate.humidity}
					<div class="stat"><span class="v">{fmt(num(climate.humidity))}<small>%</small></span><span class="l">humidity</span></div>
				{/if}
			</section>
		{:else if key === 'lighting'}
			<section class="light-head">
				<span class="count">{lightsOn.length}<small>/{lightList.length} on</small></span>
				{#if lightsOn.length > 0}
					<button class="quick" onclick={allLightsOff}>All off</button>
				{/if}
			</section>
			<div class="controls">
				{#each lightList as l (l.id)}
					<ControlTile entity={l} />
				{/each}
			</div>
		{:else if key === 'access' && access}
			{#if access.movers.length > 0}
				<div class="controls movers">
					{#each access.movers as m (m.id)}
						<ControlTile entity={m} wide={access.movers.length === 1} />
					{/each}
				</div>
			{/if}
			{#if access.openings.length > 0}
				<section class="inventory">
					<h3>Openings</h3>
					{#each access.openings as e (e.id)}
						<EntityRow entity={e} />
					{/each}
				</section>
			{/if}
		{:else if key === 'vehicle' && vehicle}
			<section class="hero">
				<Gauge value={num(vehicle.fuel)} label="fuel" tone={levelTone(num(vehicle.fuel), { warnBelow: 15 })} />
				{#if vehicle.starter}
					<div class="stat"><span class="v">{fmt(num(vehicle.starter), 1)}<small>V</small></span><span class="l">starter battery</span></div>
				{/if}
			</section>
		{/if}

		{#if controls.length > 0}
			<div class="controls">
				{#each controls as c (c.id)}
					<ControlTile entity={c} />
				{/each}
			</div>
		{/if}

		{#if byDevice.length > 0}
			<section class="inventory">
				{#each byDevice as group (group.name)}
					<h3>{group.name}</h3>
					{#each group.items as e (e.id)}
						<EntityRow entity={e} />
					{/each}
				{/each}
			</section>
		{/if}

		{#if key === 'climate' && list.some((e) => typeof e.kind === 'object' && kindName(e.kind) === 'climate')}
			<p class="note">Thermostat control lands with the climate command set; readings shown meanwhile.</p>
		{/if}
	{/if}
</div>

<style>
	.head {
		display: flex;
		align-items: center;
		gap: 0.6rem;
		margin-bottom: 1.1rem;
		color: var(--amber);
	}
	.back {
		display: grid;
		place-items: center;
		width: 44px;
		height: 44px;
		border-radius: 12px;
		color: var(--dim);
	}
	.back:hover {
		background: var(--surface);
		color: var(--text);
	}
	h1 {
		margin: 0;
		font-size: 1.25rem;
		font-weight: 600;
		color: var(--text);
	}
	.chips {
		display: flex;
		gap: 0.4rem;
		margin-left: auto;
	}
	.chip {
		font-family: var(--font-data);
		font-size: 0.7rem;
		text-transform: uppercase;
		color: var(--lamp-stale);
		border: 1px solid color-mix(in srgb, var(--lamp-stale) 55%, transparent);
		border-radius: 999px;
		padding: 0.2rem 0.55rem;
	}
	.chip.crit {
		color: var(--lamp-unavailable);
		border-color: color-mix(in srgb, var(--lamp-unavailable) 65%, transparent);
	}

	.hero {
		display: flex;
		align-items: center;
		gap: 2.2rem;
		background: var(--surface);
		border: 1px solid var(--line);
		border-radius: 16px;
		padding: 1.3rem 1.6rem;
		margin-bottom: 1rem;
		flex-wrap: wrap;
	}
	.stack-hero {
		align-items: stretch;
	}
	.tanks {
		flex: 2;
		display: flex;
		flex-direction: column;
		gap: 1rem;
		justify-content: center;
		min-width: 220px;
	}
	.hero-control {
		flex: 1;
		min-width: 210px;
		display: flex;
		flex-direction: column;
		justify-content: center;
	}
	.hero-stats {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(130px, 1fr));
		gap: 1rem 2rem;
		flex: 1;
	}
	.stat {
		display: flex;
		flex-direction: column;
	}
	.v {
		font-family: var(--font-display);
		font-weight: 600;
		font-size: 2rem;
		line-height: 1.1;
	}
	.v.giant {
		font-size: 3.6rem;
	}
	.v small {
		font-family: var(--font-data);
		font-size: 0.8rem;
		color: var(--dim);
		font-weight: 400;
		margin-left: 0.2rem;
	}
	.l {
		font-size: 0.7rem;
		text-transform: uppercase;
		letter-spacing: 0.1em;
		color: var(--faint);
	}

	.light-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 0.9rem;
	}
	.count {
		font-family: var(--font-display);
		font-weight: 600;
		font-size: 2rem;
	}
	.count small {
		font-family: var(--font-data);
		font-size: 0.85rem;
		color: var(--dim);
		font-weight: 400;
	}
	.quick {
		min-height: 46px;
		padding: 0 1.2rem;
		background: var(--surface-2);
		border: 1px solid var(--line);
		border-radius: 10px;
		font-size: 0.9rem;
		font-weight: 500;
		color: var(--text);
		cursor: pointer;
	}
	.quick:hover {
		border-color: var(--amber);
	}

	.controls {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(230px, 1fr));
		gap: 0.7rem;
		margin-bottom: 1.2rem;
	}
	.controls.movers {
		grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
	}

	.inventory h3 {
		font-size: 0.72rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.12em;
		color: var(--dim);
		margin: 1.2rem 0 0.3rem;
	}
	.none {
		color: var(--dim);
		margin-top: 2rem;
	}
	.none a {
		color: var(--amber);
	}
	.note {
		color: var(--faint);
		font-size: 0.8rem;
		margin-top: 1rem;
	}
</style>
