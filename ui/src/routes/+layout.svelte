<script lang="ts">
	import '@fontsource/barlow/400.css';
	import '@fontsource/barlow/500.css';
	import '@fontsource/barlow/600.css';
	import '@fontsource/ibm-plex-mono/400.css';
	import '@fontsource/ibm-plex-mono/500.css';
	import '../app.css';
	import favicon from '$lib/assets/favicon.svg';
	import { onMount } from 'svelte';
	import { hub, start } from '$lib/hub.svelte';

	let { children } = $props();
	onMount(start);
</script>

<svelte:head>
	<title>vanifold</title>
	<link rel="icon" href={favicon} />
</svelte:head>

<div class="shell">
	<header class="topbar">
		<div class="brand">
			<span class="mark"></span>
			<span class="word">vanifold</span>
		</div>
		<div class="link" data-conn={hub.conn}>
			{#if hub.conn === 'demo'}demo{:else if hub.conn === 'online'}link up{:else if hub.conn === 'offline'}link down{:else}connecting{/if}
			<span class="conn-lamp"></span>
		</div>
	</header>

	{@render children()}

	{#if hub.notice}
		<div class="notice" role="status">{hub.notice}</div>
	{/if}
</div>

<style>
	.shell {
		max-width: 1100px;
		margin: 0 auto;
		padding: 0 1.2rem 4rem;
	}

	.topbar {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 1.1rem 0 0.4rem;
	}

	.brand {
		display: flex;
		align-items: center;
		gap: 0.55rem;
	}
	.mark {
		width: 10px;
		height: 10px;
		background: var(--amber);
		border-radius: 2px;
		box-shadow: 0 0 8px var(--amber-soft);
	}
	.word {
		font-weight: 600;
		font-size: 1.05rem;
		letter-spacing: 0.04em;
	}

	.link {
		display: flex;
		align-items: center;
		gap: 0.45rem;
		font-family: var(--font-data);
		font-size: 0.75rem;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--dim);
	}
	.conn-lamp {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: var(--faint);
	}
	.link[data-conn='online'] .conn-lamp {
		background: var(--lamp-live);
		box-shadow: 0 0 6px var(--lamp-live);
	}
	.link[data-conn='offline'] .conn-lamp {
		background: var(--lamp-unavailable);
		animation: throb 1.2s infinite;
	}
	.link[data-conn='demo'] .conn-lamp {
		background: var(--amber);
	}
	@keyframes throb {
		50% {
			opacity: 0.3;
		}
	}

	.notice {
		position: fixed;
		bottom: 1.2rem;
		left: 50%;
		transform: translateX(-50%);
		background: var(--surface-2);
		border: 1px solid var(--lamp-unavailable);
		border-radius: 8px;
		padding: 0.55rem 1rem;
		font-size: 0.85rem;
	}
</style>
