<script lang="ts">
	import { hub, href } from '$lib/hub.svelte';
	import { PANELS, ofSubsystem } from '$lib/panels';
	import EntityRow from '$lib/components/EntityRow.svelte';
	import Icon from '$lib/components/Icon.svelte';

	const devices = $derived(
		Object.values(hub.devices)
			.map((d) => ({
				...d,
				count: Object.values(hub.entities).filter((e) => e.device_id === d.id).length
			}))
			.filter((d) => d.count > 0)
			.sort((a, b) => a.name.localeCompare(b.name))
	);
</script>

<div class="system">
	<header class="head">
		<a class="back" href={href('/')} aria-label="Back to overview"><Icon name="back" /></a>
		<Icon name="system" size={20} />
		<h1>System</h1>
		<button class="qbtn" onclick={() => (hub.quarantineOpen = true)}>
			Quarantine <b>{hub.quarantine.length}</b>
		</button>
	</header>

	<section>
		<h3>Devices</h3>
		{#each devices as d (d.id)}
			<div class="device">
				<span class="dname">{d.name}</span>
				<span class="dmeta">{[d.manufacturer, d.model].filter(Boolean).join(' ') || '—'}</span>
				<span class="dcount">{d.count} entit{d.count === 1 ? 'y' : 'ies'}</span>
			</div>
		{:else}
			<p class="none">No devices yet.</p>
		{/each}
	</section>

	<section>
		<h3>All entities</h3>
		{#each PANELS as p (p.key)}
			{@const list = ofSubsystem(hub.entities, p.key)}
			{#if list.length > 0}
				<h4>{p.label}</h4>
				{#each list as e (e.id)}
					<EntityRow entity={e} />
				{/each}
			{/if}
		{/each}
	</section>
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
	.qbtn {
		margin-left: auto;
		min-height: 44px;
		padding: 0 1rem;
		background: var(--surface-2);
		border: 1px solid var(--line);
		border-radius: 10px;
		color: var(--text);
		font-size: 0.88rem;
		cursor: pointer;
	}
	.qbtn b {
		color: var(--amber);
	}
	.qbtn:hover {
		border-color: var(--amber);
	}

	h3 {
		font-size: 0.72rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.12em;
		color: var(--dim);
		margin: 1.4rem 0 0.5rem;
	}
	h4 {
		font-size: 0.7rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.1em;
		color: var(--amber);
		margin: 1rem 0 0.2rem;
	}
	.device {
		display: flex;
		align-items: center;
		gap: 1rem;
		min-height: 44px;
		border-bottom: 1px solid var(--line);
		font-size: 0.92rem;
	}
	.dname {
		flex: 1;
		font-weight: 500;
	}
	.dmeta {
		color: var(--dim);
		font-size: 0.8rem;
	}
	.dcount {
		color: var(--faint);
		font-family: var(--font-data);
		font-size: 0.75rem;
	}
	.none {
		color: var(--faint);
	}
</style>
