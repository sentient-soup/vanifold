<script lang="ts">
	import '@fontsource/barlow/400.css';
	import '@fontsource/barlow/500.css';
	import '@fontsource/barlow/600.css';
	import '@fontsource/barlow-condensed/500.css';
	import '@fontsource/barlow-condensed/600.css';
	import '@fontsource/ibm-plex-mono/400.css';
	import '@fontsource/ibm-plex-mono/500.css';
	import '../app.css';
	import favicon from '$lib/assets/favicon.svg';
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { hub, start, href } from '$lib/hub.svelte';
	import { computeAlerts, ofSubsystem, CORE_PANELS, PANELS } from '$lib/panels';
	import DetailPanel from '$lib/components/DetailPanel.svelte';
	import QuarantinePanel from '$lib/components/QuarantinePanel.svelte';
	import Icon from '$lib/components/Icon.svelte';

	let { children } = $props();
	onMount(start);

	const alerts = $derived(computeAlerts(hub.entities, hub.now));
	const clock = $derived(
		new Date(hub.now).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
	);
	const navPanels = $derived(
		PANELS.filter(
			(p) => CORE_PANELS.includes(p.key) || ofSubsystem(hub.entities, p.key).length > 0
		)
	);
	const active = $derived(page.url.pathname);
</script>

<svelte:head>
	<title>vanifold</title>
	<link rel="icon" href={favicon} />
	<meta name="theme-color" content="#15171c" />
</svelte:head>

<svelte:window
	onkeydown={(ev) => ev.key === 'Escape' && ((hub.selected = null), (hub.quarantineOpen = false))}
/>

<div class="app">
	<nav class="rail" aria-label="Subsystems">
		<a class="brand" href={href('/')} aria-label="Home">
			<span class="mark"></span>
		</a>
		<a class="nav-item" class:active={active === '/'} href={href('/')} aria-label="Home">
			<Icon name="home" />
		</a>
		{#each navPanels as p (p.key)}
			<a
				class="nav-item"
				class:active={active === `/p/${p.key}`}
				href={href(`/p/${p.key}`)}
				aria-label={p.label}
			>
				<Icon name={p.key} />
				{#if alerts.some((a) => a.panel === p.key)}
					<span class="dot" class:crit={alerts.some((a) => a.panel === p.key && a.level === 'crit')}></span>
				{/if}
			</a>
		{/each}
		<div class="spacer"></div>
		<a class="nav-item" class:active={active === '/system'} href={href('/system')} aria-label="System">
			<Icon name="system" />
			{#if hub.quarantine.length > 0}<span class="dot"></span>{/if}
		</a>
	</nav>

	<div class="frame">
		<header class="strip">
			<span class="clock">{clock}</span>
			<div class="strip-right">
				{#each alerts.slice(0, 3) as a (a.text)}
					<a class="alert-chip {a.level}" href={href(`/p/${a.panel}`)}>
						<Icon name="alert" size={13} />
						{a.text}
					</a>
				{/each}
				<span class="link" data-conn={hub.conn}>
					{#if hub.conn === 'demo'}demo{:else if hub.conn === 'online'}link up{:else if hub.conn === 'offline'}link down{:else}connecting{/if}
					<span class="conn-lamp"></span>
				</span>
			</div>
		</header>

		<main>
			{@render children()}
		</main>
	</div>

	<DetailPanel />
	<QuarantinePanel />

	{#if hub.notice}
		<div class="notice" role="status">{hub.notice}</div>
	{/if}
</div>

<style>
	.app {
		display: flex;
		min-height: 100dvh;
	}

	/* nav rail: left on wide, bottom bar on narrow */
	.rail {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.35rem;
		padding: 0.8rem 0.55rem;
		border-right: 1px solid var(--line);
		background: var(--surface);
		position: sticky;
		top: 0;
		height: 100dvh;
	}
	.brand {
		display: grid;
		place-items: center;
		width: 46px;
		height: 40px;
		margin-bottom: 0.4rem;
	}
	.mark {
		width: 11px;
		height: 11px;
		background: var(--amber);
		border-radius: 3px;
		box-shadow: 0 0 10px var(--amber-soft);
	}
	.nav-item {
		position: relative;
		display: grid;
		place-items: center;
		width: 46px;
		height: 46px;
		border-radius: 12px;
		color: var(--dim);
	}
	.nav-item:hover {
		color: var(--text);
		background: var(--surface-2);
	}
	.nav-item.active {
		color: var(--amber);
		background: var(--amber-soft);
	}
	.dot {
		position: absolute;
		top: 8px;
		right: 8px;
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: var(--lamp-stale);
	}
	.dot.crit {
		background: var(--lamp-unavailable);
	}
	.spacer {
		flex: 1;
	}

	.frame {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
	}

	.strip {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 1rem;
		padding: 0.55rem 1.2rem;
		border-bottom: 1px solid var(--line);
	}
	.clock {
		font-family: var(--font-display);
		font-weight: 600;
		font-size: 1.2rem;
		letter-spacing: 0.03em;
	}
	.strip-right {
		display: flex;
		align-items: center;
		gap: 0.7rem;
		min-width: 0;
	}
	.alert-chip {
		display: flex;
		align-items: center;
		gap: 0.3rem;
		font-family: var(--font-data);
		font-size: 0.72rem;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		color: var(--lamp-stale);
		border: 1px solid color-mix(in srgb, var(--lamp-stale) 55%, transparent);
		border-radius: 999px;
		padding: 0.22rem 0.6rem;
		white-space: nowrap;
	}
	.alert-chip.crit {
		color: var(--lamp-unavailable);
		border-color: color-mix(in srgb, var(--lamp-unavailable) 65%, transparent);
	}
	.link {
		display: flex;
		align-items: center;
		gap: 0.45rem;
		font-family: var(--font-data);
		font-size: 0.72rem;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--dim);
		white-space: nowrap;
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

	main {
		flex: 1;
		padding: 1.1rem 1.2rem 2rem;
		max-width: 1240px;
		width: 100%;
		margin: 0 auto;
	}

	.notice {
		position: fixed;
		bottom: 4.6rem;
		left: 50%;
		transform: translateX(-50%);
		background: var(--surface-2);
		border: 1px solid var(--lamp-unavailable);
		border-radius: 8px;
		padding: 0.55rem 1rem;
		font-size: 0.85rem;
		z-index: 20;
	}

	@media (max-width: 760px) {
		.app {
			flex-direction: column-reverse;
		}
		.rail {
			flex-direction: row;
			height: auto;
			width: 100%;
			position: fixed;
			bottom: 0;
			top: auto;
			border-right: none;
			border-top: 1px solid var(--line);
			padding: 0.3rem 0.6rem;
			justify-content: space-around;
			z-index: 5;
			overflow-x: auto;
		}
		.brand {
			display: none;
		}
		.frame {
			padding-bottom: 4rem;
		}
		.clock {
			font-size: 1rem;
		}
	}
</style>
