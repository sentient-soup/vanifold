<script lang="ts">
	import { hub, refreshQuarantine } from '$lib/hub.svelte';

	function close() {
		hub.quarantineOpen = false;
	}
	$effect(() => {
		if (hub.quarantineOpen) refreshQuarantine();
	});
</script>

{#if hub.quarantineOpen}
	<div class="scrim" onclick={close} role="presentation"></div>
	<aside class="drawer" aria-label="Quarantined devices">
		<header>
			<h2>Quarantine</h2>
			<button class="close" onclick={close} aria-label="Close quarantine">×</button>
		</header>
		<p class="blurb">
			Discovery payloads the hub could not accept. Nothing is dropped silently: each entry keeps
			its topic, payload, and the reason it was refused.
		</p>
		{#if hub.quarantine.length === 0}
			<p class="empty">Empty. Every announced device was ingested.</p>
		{:else}
			{#each hub.quarantine as item (item.topic)}
				<article>
					<div class="reason">{item.reason}</div>
					<div class="topic">{item.topic}</div>
					<pre>{item.payload}</pre>
				</article>
			{/each}
		{/if}
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
		width: min(460px, 100vw);
		background: var(--bg);
		border-left: 1px solid var(--line);
		padding: 1.1rem 1.2rem;
		overflow-y: auto;
		z-index: 11;
	}
	header {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}
	h2 {
		margin: 0;
		font-size: 1.1rem;
		font-weight: 600;
	}
	.close {
		background: none;
		border: none;
		font-size: 1.5rem;
		color: var(--dim);
		cursor: pointer;
		line-height: 1;
	}
	.blurb,
	.empty {
		color: var(--dim);
		font-size: 0.85rem;
	}
	article {
		background: var(--surface);
		border: 1px solid var(--line);
		border-radius: var(--radius);
		padding: 0.7rem 0.8rem;
		margin-bottom: 0.7rem;
	}
	.reason {
		color: var(--lamp-stale);
		font-weight: 500;
		font-size: 0.9rem;
	}
	.topic {
		font-family: var(--font-data);
		font-size: 0.72rem;
		color: var(--faint);
		margin: 0.25rem 0 0.4rem;
		word-break: break-all;
	}
	pre {
		margin: 0;
		font-family: var(--font-data);
		font-size: 0.72rem;
		color: var(--dim);
		white-space: pre-wrap;
		word-break: break-all;
		max-height: 8rem;
		overflow-y: auto;
	}
</style>
