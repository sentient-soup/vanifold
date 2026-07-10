// Mirrors core/src/model.rs serialization. Keep in sync by hand until ts-rs
// codegen earns its place.

export type Value = number | boolean | string;
export type Quality = 'live' | 'retained' | 'stale' | 'unavailable';
export type EntityKind =
	| 'sensor'
	| 'binary_sensor'
	| 'switch'
	| 'light'
	| 'cover'
	| { deferred: string };

export interface EntityState {
	value: Value;
	updated_at: number;
	retained: boolean;
}

export interface Entity {
	id: string;
	device_id: string;
	kind: EntityKind;
	name: string;
	unit?: string | null;
	device_class?: string | null;
	criticality: 'info' | 'comfort' | 'safety';
	subsystem?: string | null;
	state?: EntityState | null;
	available?: boolean | null;
	attributes: Record<string, unknown>;
	/** Present on REST/WS snapshots; recomputed client-side afterwards. */
	quality?: Quality;
	command?: CommandCfg | null;
}

export type CommandCfg =
	| { Switch: object }
	| { Light: { Basic: { brightness_command_topic?: string | null } } | { Json: { brightness: boolean } } }
	| { Cover: { set_position_topic?: string | null } };

export interface Device {
	id: string;
	name: string;
	manufacturer?: string | null;
	model?: string | null;
}

/** Commands sent over the socket ({command: "turn_on"} etc.). */
export type Command =
	| { command: 'turn_on' }
	| { command: 'turn_off' }
	| { command: 'set_brightness'; brightness: number }
	| { command: 'open' }
	| { command: 'close' }
	| { command: 'stop' };

export function kindName(k: EntityKind): string {
	return typeof k === 'string' ? k : k.deferred;
}

export function supportsBrightness(e: Entity): boolean {
	const c = e.command;
	if (!c || !('Light' in c)) return false;
	const l = c.Light;
	if ('Basic' in l) return !!l.Basic.brightness_command_topic;
	return l.Json.brightness;
}

export const STALE_AFTER_MS = 900_000;

/** Client-side quality: same ladder as core/src/model.rs. */
export function quality(e: Entity, now: number): Quality {
	if (e.available === false) return 'unavailable';
	if (!e.state) return 'stale';
	if (e.state.retained) return 'retained';
	if (now - e.state.updated_at > STALE_AFTER_MS) return 'stale';
	return 'live';
}
