<script lang="ts">
	import EntityCard from '$lib/components/EntityCard.svelte';
	import { hub } from '$lib/hub.svelte';
	import type { Entity } from '$lib/types';

	const SUBSYSTEM_ORDER = ['power', 'climate', 'plumbing', 'vehicle', 'computer', 'misc'];

	const groups = $derived.by(() => {
		const bySubsystem = new Map<string, Entity[]>();
		for (const e of Object.values(hub.entities)) {
			const key = e.subsystem ?? 'misc';
			if (!bySubsystem.has(key)) bySubsystem.set(key, []);
			bySubsystem.get(key)!.push(e);
		}
		for (const list of bySubsystem.values()) {
			list.sort((a, b) => a.device_id.localeCompare(b.device_id) || a.name.localeCompare(b.name));
		}
		const known = SUBSYSTEM_ORDER.filter((s) => bySubsystem.has(s));
		const extra = [...bySubsystem.keys()].filter((s) => !SUBSYSTEM_ORDER.includes(s)).sort();
		return [...known, ...extra].map((s) => ({ name: s, entities: bySubsystem.get(s)! }));
	});

	const empty = $derived(Object.keys(hub.entities).length === 0);
</script>

{#if empty}
	<section class="empty">
		<h2>No devices yet</h2>
		<p>
			Anything that speaks Home Assistant MQTT discovery appears here on its own: point it at the
			broker and watch. To fake a device from the hub:
		</p>
		<pre>mosquitto_pub -u vanifold-node -P &lt;password&gt; -r \
  -t 'homeassistant/sensor/demo/config' \
  -m '&#123;"uniq_id":"demo1","name":"Demo","stat_t":"demo/val","unit_of_meas":"W"&#125;'</pre>
		<p class="hint">Or open <a href="?demo">the demo dashboard</a> to explore the UI without hardware.</p>
	</section>
{:else}
	{#each groups as group (group.name)}
		<section>
			<h2 class="eyebrow">{group.name}</h2>
			<div class="grid">
				{#each group.entities as entity (entity.id)}
					<EntityCard {entity} />
				{/each}
			</div>
		</section>
	{/each}
{/if}

<style>
	.eyebrow {
		font-size: 0.72rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.14em;
		color: var(--amber);
		border-bottom: 1px solid var(--line);
		padding: 1.4rem 0 0.45rem;
		margin: 0 0 0.8rem;
	}

	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(230px, 1fr));
		gap: 0.7rem;
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
	.empty pre {
		display: inline-block;
		text-align: left;
		background: var(--surface);
		border: 1px solid var(--line);
		border-radius: var(--radius);
		padding: 0.9rem 1.1rem;
		font-size: 0.78rem;
		overflow-x: auto;
		max-width: 100%;
	}
	.empty a {
		color: var(--amber);
	}
	.hint {
		font-size: 0.85rem;
	}
</style>
