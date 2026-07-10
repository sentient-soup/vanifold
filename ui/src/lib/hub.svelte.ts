// Live connection to vanifold-core: snapshot-then-stream over /api/ws,
// command dispatch with pending tracking (confirmed-not-optimistic:
// pending clears on the state echo, not on send).

import type { Command, Device, Entity, EntityState } from './types';
import { demoEntities, demoDevices, demoSimulate } from './demo';

type Conn = 'connecting' | 'online' | 'offline' | 'demo';

export const hub = $state({
	conn: 'connecting' as Conn,
	entities: {} as Record<string, Entity>,
	devices: {} as Record<string, Device>,
	/** entity_id -> send timestamp; presence = command in flight */
	pending: {} as Record<string, number>,
	notice: null as string | null,
	now: Date.now()
});

let ws: WebSocket | null = null;
let demo = false;
let seq = 0;

export function start() {
	demo = new URLSearchParams(location.search).has('demo');
	if (demo) {
		hub.conn = 'demo';
		hub.entities = demoEntities();
		hub.devices = demoDevices();
	} else {
		connect();
	}
	setInterval(() => (hub.now = Date.now()), 10_000);
}

function connect() {
	const proto = location.protocol === 'https:' ? 'wss' : 'ws';
	ws = new WebSocket(`${proto}://${location.host}/api/ws`);
	ws.onopen = () => (hub.conn = 'online');
	ws.onmessage = (m) => handle(JSON.parse(m.data));
	ws.onclose = () => {
		hub.conn = 'offline';
		setTimeout(connect, 2000);
	};
	ws.onerror = () => ws?.close();
}

type ServerMsg =
	| { type: 'snapshot'; entities: Entity[]; devices: Device[] }
	| { type: 'entity_upserted'; entity: Entity }
	| { type: 'entity_removed'; entity_id: string }
	| { type: 'state_changed'; entity_id: string; state: EntityState }
	| { type: 'availability_changed'; entity_id: string; available: boolean }
	| { type: 'attribute_changed'; entity_id: string; key: string; value: unknown }
	| { type: 'result'; id?: string; ok: boolean; reason?: string };

function handle(msg: ServerMsg) {
	switch (msg.type) {
		case 'snapshot': {
			hub.entities = Object.fromEntries(msg.entities.map((e) => [e.id, e]));
			hub.devices = Object.fromEntries(msg.devices.map((d) => [d.id, d]));
			break;
		}
		case 'entity_upserted':
			hub.entities[msg.entity.id] = msg.entity;
			break;
		case 'entity_removed':
			delete hub.entities[msg.entity_id];
			break;
		case 'state_changed': {
			const e = hub.entities[msg.entity_id];
			if (e) e.state = msg.state;
			delete hub.pending[msg.entity_id]; // the echo confirms the command
			break;
		}
		case 'availability_changed': {
			const e = hub.entities[msg.entity_id];
			if (e) e.available = msg.available;
			break;
		}
		case 'attribute_changed': {
			const e = hub.entities[msg.entity_id];
			if (e) e.attributes[msg.key] = msg.value;
			break;
		}
		case 'result':
			if (!msg.ok) flash(msg.reason ?? 'command rejected');
			break;
	}
}

export function send(entityId: string, command: Command) {
	if (demo) {
		demoSimulate(hub.entities, entityId, command);
		return;
	}
	if (hub.conn !== 'online' || !ws) {
		flash('link down, command not sent');
		return;
	}
	hub.pending[entityId] = Date.now();
	ws.send(JSON.stringify({ type: 'command', id: `c${seq++}`, entity_id: entityId, ...command }));
	// Ack timeout mirrors the server contract (5s): no echo means failure.
	setTimeout(() => {
		if (hub.pending[entityId]) {
			delete hub.pending[entityId];
			flash(`no response from ${hub.entities[entityId]?.name ?? entityId}`);
		}
	}, 5000);
}

let noticeTimer: ReturnType<typeof setTimeout>;
function flash(text: string) {
	hub.notice = text;
	clearTimeout(noticeTimer);
	noticeTimer = setTimeout(() => (hub.notice = null), 4000);
}
