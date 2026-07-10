<script lang="ts">
	import { goto } from '$app/navigation';
	import { hub, href, send } from '$lib/hub.svelte';
	import {
		accessSummary,
		climateHero,
		computeAlerts,
		fmt,
		levelTone,
		lights,
		num,
		ofSubsystem,
		powerHero,
		vehicleHero,
		waterHero
	} from '$lib/panels';
	import Gauge from '$lib/components/Gauge.svelte';
	import TankBar from '$lib/components/TankBar.svelte';
	import Icon from '$lib/components/Icon.svelte';

	const power = $derived(powerHero(ofSubsystem(hub.entities, 'power')));
	const water = $derived(waterHero(ofSubsystem(hub.entities, 'water')));
	const climate = $derived(climateHero(ofSubsystem(hub.entities, 'climate')));
	const access = $derived(accessSummary(ofSubsystem(hub.entities, 'access')));
	const vehicle = $derived(vehicleHero(ofSubsystem(hub.entities, 'vehicle')));
	const lightList = $derived(lights(ofSubsystem(hub.entities, 'lighting')));
	const lightsOn = $derived(lightList.filter((l) => l.state?.value === true));
	const media = $derived(ofSubsystem(hub.entities, 'media'));
	const misc = $derived(ofSubsystem(hub.entities, 'misc'));
	const alerts = $derived(computeAlerts(hub.entities, hub.now));
	const empty = $derived(Object.keys(hub.entities).length === 0);

	const socTone = $derived(levelTone(num(power.soc), { warnBelow: 20, critBelow: 10 }));

	function open(key: string) {
		goto(href(`/p/${key}`));
	}
	function tileKey(ev: KeyboardEvent, key: string) {
		if (ev.key === 'Enter' || ev.key === ' ') {
			ev.preventDefault();
			open(key);
		}
	}
	function allLightsOff(ev: MouseEvent) {
		ev.stopPropagation();
		for (const l of lightsOn) send(l.id, { command: 'turn_off' });
	}
</script>

{#if empty}
	<section class="empty">
		<h2>No devices yet</h2>
		<p>
			Anything that speaks Home Assistant MQTT discovery appears here on its own: point it at the
			broker and watch. Or open <a href="/?demo">the demo dashboard</a> to explore without hardware.
		</p>
	</section>
{:else}
	<div class="board">
		<!-- POWER: the hero. Energy is the resource everything else depends on. -->
		<div
			class="tile power"
			role="link"
			tabindex="0"
			aria-label="Power panel"
			onclick={() => open('power')}
			onkeydown={(ev) => tileKey(ev, 'power')}
		>
			<header><Icon name="power" size={16} /><h2>Power</h2></header>
			<div class="power-body">
				<Gauge value={num(power.soc)} label="battery" tone={socTone} />
				<div class="power-stats">
					<div class="stat">
						<span class="stat-v">{fmt(num(power.voltage), 1)}<small>V</small></span>
						<span class="stat-l">battery</span>
					</div>
					{#if power.solar}
						<div class="stat">
							<span class="stat-v">{fmt(num(power.solar))}<small>W</small></span>
							<span class="stat-l">solar</span>
						</div>
					{/if}
					<div class="stat">
						<span class="stat-v">{fmt(power.wattsTotal)}<small>W</small></span>
						<span class="stat-l">measured</span>
					</div>
					{#if power.batteryTemp}
						<div class="stat">
							<span class="stat-v">{fmt(num(power.batteryTemp), 1)}<small>°</small></span>
							<span class="stat-l">batt temp</span>
						</div>
					{/if}
				</div>
			</div>
		</div>

		<!-- WATER -->
		<div
			class="tile"
			role="link"
			tabindex="0"
			aria-label="Water panel"
			onclick={() => open('water')}
			onkeydown={(ev) => tileKey(ev, 'water')}
		>
			<header><Icon name="water" size={16} /><h2>Water</h2></header>
			<div class="stack">
				{#each water.tanks.slice(0, 3) as t (t.entity.id)}
					<TankBar
						label={t.entity.name}
						value={num(t.entity)}
						hue={t.role === 'grey' ? 'grey' : 'water'}
						tone={t.role === 'grey'
							? levelTone(num(t.entity), { warnAbove: 85 })
							: levelTone(num(t.entity), { warnBelow: 15 })}
					/>
				{:else}
					<p class="none">no tank sensors</p>
				{/each}
				{#if water.pump}
					<div class="inline-state">
						pump <b class:on={water.pump.state?.value === true}>{water.pump.state?.value === true ? 'ON' : 'off'}</b>
					</div>
				{/if}
			</div>
		</div>

		<!-- CLIMATE -->
		<div
			class="tile"
			role="link"
			tabindex="0"
			aria-label="Climate panel"
			onclick={() => open('climate')}
			onkeydown={(ev) => tileKey(ev, 'climate')}
		>
			<header><Icon name="climate" size={16} /><h2>Climate</h2></header>
			<div class="climate-body">
				<span class="huge">{fmt(num(climate.inside), 1)}<small>°C</small></span>
				<span class="sub">inside{climate.humidity ? ` · ${fmt(num(climate.humidity))}% rh` : ''}</span>
			</div>
		</div>

		<!-- LIGHTING -->
		<div
			class="tile"
			role="link"
			tabindex="0"
			aria-label="Lighting panel"
			onclick={() => open('lighting')}
			onkeydown={(ev) => tileKey(ev, 'lighting')}
		>
			<header><Icon name="lighting" size={16} /><h2>Lighting</h2></header>
			<div class="split">
				<div class="climate-body">
					<span class="huge">{lightsOn.length}<small>/{lightList.length}</small></span>
					<span class="sub">lights on</span>
				</div>
				{#if lightsOn.length > 0}
					<button class="quick" onclick={allLightsOff}>All off</button>
				{/if}
			</div>
		</div>

		<!-- ACCESS -->
		<div
			class="tile"
			role="link"
			tabindex="0"
			aria-label="Access panel"
			onclick={() => open('access')}
			onkeydown={(ev) => tileKey(ev, 'access')}
		>
			<header><Icon name="access" size={16} /><h2>Access</h2></header>
			<div class="climate-body">
				{#if access.openings.length === 0 && access.movers.length === 0}
					<p class="none">no sensors</p>
				{:else if access.openCount === 0 && access.unknownCount === 0}
					<span class="huge secure">Secure</span>
					<span class="sub">all openings closed</span>
				{:else}
					<span class="huge warn-t">{access.openCount || '?'}</span>
					<span class="sub">{access.openCount === 1 ? 'opening is' : 'openings'} open{access.unknownCount ? `, ${access.unknownCount} unknown` : ''}</span>
				{/if}
			</div>
		</div>

		<!-- VEHICLE -->
		<div
			class="tile"
			role="link"
			tabindex="0"
			aria-label="Vehicle panel"
			onclick={() => open('vehicle')}
			onkeydown={(ev) => tileKey(ev, 'vehicle')}
		>
			<header><Icon name="vehicle" size={16} /><h2>Vehicle</h2></header>
			<div class="split">
				<Gauge value={num(vehicle.fuel)} label="fuel" size="small" tone={levelTone(num(vehicle.fuel), { warnBelow: 15 })} />
				{#if vehicle.starter}
					<div class="stat">
						<span class="stat-v">{fmt(num(vehicle.starter), 1)}<small>V</small></span>
						<span class="stat-l">starter</span>
					</div>
				{/if}
			</div>
		</div>

		{#if media.length > 0}
			<div class="tile" role="link" tabindex="0" aria-label="Media panel" onclick={() => open('media')} onkeydown={(ev) => tileKey(ev, 'media')}>
				<header><Icon name="media" size={16} /><h2>Media</h2></header>
				<p class="none">{media.length} device{media.length === 1 ? '' : 's'}</p>
			</div>
		{/if}
		{#if misc.length > 0}
			<div class="tile" role="link" tabindex="0" aria-label="Other panel" onclick={() => open('misc')} onkeydown={(ev) => tileKey(ev, 'misc')}>
				<header><Icon name="misc" size={16} /><h2>Other</h2></header>
				<p class="none">{misc.length} unassigned entit{misc.length === 1 ? 'y' : 'ies'}</p>
			</div>
		{/if}
	</div>

	{#if alerts.length > 3}
		<p class="more-alerts">{alerts.length - 3} more alert{alerts.length - 3 === 1 ? '' : 's'} in panels</p>
	{/if}
{/if}

<style>
	.board {
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		grid-template-areas:
			'power power water climate'
			'power power lighting access'
			'vehicle vehicle media misc2';
		gap: 0.8rem;
	}
	.tile.power {
		grid-area: power;
	}
	.tile:nth-of-type(2) {
		grid-area: water;
	}
	.tile:nth-of-type(3) {
		grid-area: climate;
	}
	.tile:nth-of-type(4) {
		grid-area: lighting;
	}
	.tile:nth-of-type(5) {
		grid-area: access;
	}
	.tile:nth-of-type(6) {
		grid-area: vehicle;
	}

	@media (max-width: 1020px) {
		.board {
			grid-template-columns: 1fr 1fr;
			grid-template-areas:
				'power power'
				'water climate'
				'lighting access'
				'vehicle media'
				'misc2 misc2';
		}
	}
	@media (max-width: 620px) {
		.board {
			grid-template-columns: 1fr;
			grid-template-areas: 'power' 'water' 'climate' 'lighting' 'access' 'vehicle' 'media' 'misc2';
		}
	}

	.tile {
		background: var(--surface);
		border: 1px solid var(--line);
		border-radius: 16px;
		padding: 1rem 1.1rem;
		display: flex;
		flex-direction: column;
		gap: 0.7rem;
		cursor: pointer;
		min-height: 9.5rem;
		transition: border-color 150ms;
		container-type: inline-size;
	}
	.tile:hover,
	.tile:focus-visible {
		border-color: var(--faint);
	}

	header {
		display: flex;
		align-items: center;
		gap: 0.45rem;
		color: var(--amber);
	}
	h2 {
		margin: 0;
		font-size: 0.75rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.14em;
		color: var(--dim);
	}

	.power-body {
		display: flex;
		align-items: center;
		gap: 1.4rem;
		flex: 1;
		flex-wrap: wrap;
	}
	.power-stats {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 0.9rem 1.6rem;
		flex: 1;
		min-width: 150px;
	}
	.stat {
		display: flex;
		flex-direction: column;
	}
	.stat-v {
		font-family: var(--font-display);
		font-weight: 600;
		font-size: 1.7rem;
		line-height: 1.1;
	}
	.stat-v small,
	.huge small {
		font-family: var(--font-data);
		font-size: 0.75rem;
		color: var(--dim);
		font-weight: 400;
		margin-left: 0.15rem;
	}
	.stat-l {
		font-size: 0.68rem;
		text-transform: uppercase;
		letter-spacing: 0.1em;
		color: var(--faint);
	}

	.stack {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		flex: 1;
		justify-content: center;
	}
	.inline-state {
		font-size: 0.78rem;
		color: var(--dim);
		text-transform: uppercase;
		letter-spacing: 0.08em;
	}
	.inline-state b {
		color: var(--faint);
		font-weight: 600;
	}
	.inline-state b.on {
		color: var(--amber);
	}

	.climate-body {
		display: flex;
		flex-direction: column;
		justify-content: center;
		flex: 1;
		gap: 0.15rem;
	}
	.huge {
		font-family: var(--font-display);
		font-weight: 600;
		font-size: 2.6rem;
		line-height: 1;
	}
	.huge.secure {
		color: var(--lamp-live);
	}
	.huge.warn-t {
		color: var(--lamp-stale);
	}
	.sub {
		font-size: 0.78rem;
		color: var(--dim);
	}

	.split {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.8rem;
		flex: 1;
	}
	.quick {
		min-height: 46px;
		padding: 0 1.1rem;
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

	.none {
		color: var(--faint);
		font-size: 0.85rem;
		margin: auto 0;
	}

	.more-alerts {
		color: var(--dim);
		font-size: 0.8rem;
		margin-top: 0.8rem;
	}

	.empty {
		margin-top: 4rem;
		text-align: center;
		color: var(--dim);
	}
	.empty h2 {
		color: var(--text);
		font-weight: 500;
	}
	.empty a {
		color: var(--amber);
	}
</style>
