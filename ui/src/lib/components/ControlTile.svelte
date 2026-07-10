<script lang="ts">
	import { hub, send } from '$lib/hub.svelte';
	import { kindName, quality, supportsBrightness, type Entity } from '$lib/types';

	let { entity, wide = false }: { entity: Entity; wide?: boolean } = $props();

	const kind = $derived(kindName(entity.kind));
	const q = $derived(quality(entity, hub.now));
	const disabled = $derived(q === 'unavailable');
	const pending = $derived(entity.id in hub.pending);
	const isOn = $derived(entity.state?.value === true);
	const brightness = $derived(Number(entity.attributes['brightness'] ?? 255));
	const coverState = $derived(typeof entity.state?.value === 'string' ? entity.state.value : null);
	const moving = $derived(coverState === 'opening' || coverState === 'closing');
	const safety = $derived(entity.criticality === 'safety');

	function toggle() {
		send(entity.id, { command: isOn ? 'turn_off' : 'turn_on' });
	}
	function detail(ev: MouseEvent) {
		ev.stopPropagation();
		hub.selected = entity.id;
	}
</script>

{#if kind === 'cover'}
	<div class="tile cover" class:wide class:safety class:pending>
		<div class="head">
			<button class="name" onclick={detail}>{entity.name}</button>
			{#if safety}<span class="tag">safety</span>{/if}
		</div>
		<div class="cover-state" class:moving>{coverState ?? '—'}</div>
		<div class="cover-actions">
			<button {disabled} onclick={() => send(entity.id, { command: 'open' })}>Open</button>
			<button {disabled} onclick={() => send(entity.id, { command: 'stop' })}>Stop</button>
			<button {disabled} onclick={() => send(entity.id, { command: 'close' })}>Close</button>
		</div>
	</div>
{:else}
	<div class="tile switch" class:wide class:on={isOn} class:pending>
		<button class="hit" onclick={toggle} {disabled} aria-pressed={isOn} aria-label="{entity.name}: turn {isOn ? 'off' : 'on'}">
			<span class="knob"></span>
			<span class="txt">
				<span class="name-txt">{entity.name}</span>
				<span class="state-txt">{disabled ? 'offline' : isOn ? 'on' : 'off'}</span>
			</span>
		</button>
		{#if kind === 'light' && supportsBrightness(entity)}
			<input
				type="range"
				min="1"
				max="255"
				value={brightness}
				{disabled}
				aria-label="{entity.name} brightness"
				onchange={(ev) => send(entity.id, { command: 'set_brightness', brightness: Number(ev.currentTarget.value) })}
			/>
		{/if}
		<button class="dots" onclick={detail} aria-label="{entity.name} details">···</button>
	</div>
{/if}

<style>
	.tile {
		background: var(--surface);
		border: 1px solid var(--line);
		border-radius: 14px;
		padding: 0.8rem 0.9rem;
		display: flex;
		flex-direction: column;
		gap: 0.6rem;
		min-height: 5.6rem;
		position: relative;
	}
	.tile.pending {
		border-color: var(--amber);
	}
	.tile.wide {
		grid-column: 1 / -1;
	}

	/* switch / light */
	.switch.on {
		border-color: color-mix(in srgb, var(--amber) 55%, var(--line));
		background: linear-gradient(160deg, var(--amber-soft), var(--surface) 55%);
	}
	.hit {
		display: flex;
		align-items: center;
		gap: 0.8rem;
		background: none;
		border: none;
		padding: 0.2rem;
		cursor: pointer;
		text-align: left;
		min-height: 44px;
		flex: 1;
	}
	.hit:disabled {
		cursor: default;
		opacity: 0.5;
	}
	.knob {
		flex: none;
		width: 2rem;
		height: 2rem;
		border-radius: 50%;
		background: var(--surface-2);
		border: 1px solid var(--line);
		transition: all 150ms;
	}
	.on .knob {
		background: var(--amber);
		border-color: var(--amber);
		box-shadow: 0 0 14px var(--amber-soft), 0 0 5px var(--amber);
	}
	.txt {
		display: flex;
		flex-direction: column;
		gap: 0.1rem;
		min-width: 0;
	}
	.name-txt {
		font-weight: 500;
		font-size: 0.95rem;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.state-txt {
		font-family: var(--font-data);
		font-size: 0.7rem;
		text-transform: uppercase;
		letter-spacing: 0.08em;
		color: var(--dim);
	}
	input[type='range'] {
		width: 100%;
		accent-color: var(--amber);
		height: 28px;
	}
	.dots {
		position: absolute;
		top: 0.35rem;
		right: 0.45rem;
		background: none;
		border: none;
		color: var(--faint);
		font-size: 0.9rem;
		cursor: pointer;
		padding: 0.3rem 0.4rem;
		line-height: 1;
	}
	.dots:hover {
		color: var(--dim);
	}

	/* cover */
	.cover.safety {
		border-color: color-mix(in srgb, var(--lamp-unavailable) 45%, var(--line));
	}
	.head {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 0.5rem;
	}
	.name {
		background: none;
		border: none;
		color: var(--text);
		font-weight: 500;
		font-size: 0.95rem;
		padding: 0;
		cursor: pointer;
		text-align: left;
	}
	.tag {
		font-family: var(--font-data);
		font-size: 0.6rem;
		text-transform: uppercase;
		letter-spacing: 0.08em;
		color: var(--lamp-unavailable);
		border: 1px solid var(--lamp-unavailable);
		border-radius: 4px;
		padding: 0.05rem 0.3rem;
	}
	.cover-state {
		font-family: var(--font-display);
		font-weight: 600;
		font-size: 1.5rem;
		text-transform: capitalize;
		line-height: 1;
	}
	.cover-state.moving {
		color: var(--amber);
		animation: pulse 1.2s ease-in-out infinite;
	}
	@keyframes pulse {
		50% {
			opacity: 0.45;
		}
	}
	.cover-actions {
		display: flex;
		gap: 0.5rem;
	}
	.cover-actions button {
		flex: 1;
		min-height: 46px;
		background: var(--surface-2);
		border: 1px solid var(--line);
		border-radius: 10px;
		font-size: 0.9rem;
		font-weight: 500;
		cursor: pointer;
	}
	.cover-actions button:hover:not(:disabled) {
		border-color: var(--amber);
	}
	.cover-actions button:disabled {
		opacity: 0.5;
		cursor: default;
	}
</style>
