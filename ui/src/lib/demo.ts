// Demo fixtures: a plausible build of the owner's actual van (48V system,
// winch-driven rear entry). Open the app with ?demo to develop and judge the
// UI with no hardware attached. Commands are simulated locally.

import type { Command, Device, Entity } from './types';

const now = () => Date.now();

function entity(partial: Partial<Entity> & Pick<Entity, 'id' | 'name' | 'kind'>): Entity {
	return {
		device_id: 'demo',
		criticality: 'info',
		attributes: {},
		available: true,
		...partial
	} as Entity;
}

export function demoDevices(): Record<string, Device> {
	const list: Device[] = [
		{ id: 'shelly3em', name: 'Shelly 3EM', manufacturer: 'Shelly', model: '3EM' },
		{ id: 'bms', name: 'House battery BMS', manufacturer: 'JBD', model: 'SP25S003' },
		{ id: 'electronics-bay', name: 'Electronics bay node', manufacturer: 'vanifold', model: 'ESP32' },
		{ id: 'rear-entry', name: 'Rear entry node', manufacturer: 'vanifold', model: 'ESP32' },
		{ id: 'scatter', name: 'Zigbee sensors', manufacturer: 'Aqara' }
	];
	return Object.fromEntries(list.map((d) => [d.id, d]));
}

export function demoEntities(): Record<string, Entity> {
	const t = now();
	const list: Entity[] = [
		entity({
			id: 'battery_voltage', name: 'Battery voltage', kind: 'sensor', unit: 'V',
			device_class: 'voltage', subsystem: 'power', device_id: 'bms',
			state: { value: 52.4, updated_at: t, retained: false }
		}),
		entity({
			id: 'battery_soc', name: 'State of charge', kind: 'sensor', unit: '%',
			device_class: 'battery', subsystem: 'power', device_id: 'bms',
			state: { value: 87, updated_at: t, retained: false }
		}),
		entity({
			id: 'solar_power', name: 'Solar input', kind: 'sensor', unit: 'W',
			device_class: 'power', subsystem: 'power', device_id: 'shelly3em',
			state: { value: 342, updated_at: t, retained: false }
		}),
		entity({
			id: 'inverter_power', name: 'Inverter load', kind: 'sensor', unit: 'W',
			device_class: 'power', subsystem: 'power', device_id: 'shelly3em',
			state: { value: 118, updated_at: t, retained: false }
		}),
		entity({
			id: 'battery_temp', name: 'Battery temp', kind: 'sensor', unit: '°C',
			device_class: 'temperature', subsystem: 'power', device_id: 'electronics-bay',
			state: { value: 18.5, updated_at: t, retained: false }
		}),
		entity({
			id: 'bay_fan', name: 'Bay fan', kind: 'switch', subsystem: 'power',
			device_id: 'electronics-bay', command: { Switch: {} },
			state: { value: false, updated_at: t, retained: false }
		}),
		entity({
			id: 'cabin_temp', name: 'Cabin temp', kind: 'sensor', unit: '°C',
			device_class: 'temperature', subsystem: 'climate', device_id: 'scatter',
			state: { value: 21.5, updated_at: t, retained: false }
		}),
		entity({
			id: 'cabin_humidity', name: 'Cabin humidity', kind: 'sensor', unit: '%',
			device_class: 'humidity', subsystem: 'climate', device_id: 'scatter',
			state: { value: 46, updated_at: t, retained: true } // retained: age unknown
		}),
		entity({
			id: 'fresh_level', name: 'Fresh water', kind: 'sensor', unit: '%',
			subsystem: 'plumbing', device_id: 'electronics-bay',
			state: { value: 63, updated_at: t, retained: false }
		}),
		entity({
			id: 'grey_level', name: 'Grey water', kind: 'sensor', unit: '%',
			subsystem: 'plumbing', device_id: 'electronics-bay',
			state: { value: 41, updated_at: t - 3_600_000, retained: false } // stale
		}),
		entity({
			id: 'water_pump', name: 'Water pump', kind: 'switch', subsystem: 'plumbing',
			device_id: 'electronics-bay', command: { Switch: {} }, criticality: 'comfort',
			state: { value: true, updated_at: t, retained: false }
		}),
		entity({
			id: 'rear_entry', name: 'Rear entry', kind: 'cover', subsystem: 'misc',
			device_id: 'rear-entry', criticality: 'safety',
			command: { Cover: {} },
			state: { value: 'closed', updated_at: t, retained: false }
		}),
		entity({
			id: 'galley_light', name: 'Galley light', kind: 'light', subsystem: 'misc',
			device_id: 'electronics-bay',
			command: { Light: { Basic: { brightness_command_topic: 'x' } } },
			state: { value: true, updated_at: t, retained: false },
			attributes: { brightness: 180 }
		}),
		entity({
			id: 'cab_door', name: 'Cab door', kind: 'binary_sensor', device_class: 'door',
			subsystem: 'misc', device_id: 'scatter',
			state: { value: false, updated_at: t, retained: false }
		}),
		entity({
			id: 'fuel_level', name: 'Fuel', kind: 'sensor', unit: '%',
			subsystem: 'vehicle', device_id: 'scatter', available: false, // unavailable
			state: { value: 58, updated_at: t - 7_200_000, retained: false }
		})
	];
	return Object.fromEntries(list.map((e) => [e.id, e]));
}

/** Synthesized history so the detail chart renders in demo mode. */
export function demoHistory(entityId: string, hours: number) {
	const e = demoEntities()[entityId];
	const base = typeof e?.state?.value === 'number' ? (e.state.value as number) : 50;
	const to = now();
	const points = [];
	for (let i = 0; i < hours * 12; i++) {
		const ts = to - hours * 3600_000 + i * 300_000;
		const v = base + Math.sin(i / 9) * base * 0.12 + Math.sin(i / 3.1) * base * 0.04;
		points.push({ ts, vmin: v - base * 0.03, vmax: v + base * 0.03, vavg: v });
	}
	return points;
}

/** Local command simulation so the demo dashboard is playable. */
export function demoSimulate(entities: Record<string, Entity>, id: string, cmd: Command) {
	const e = entities[id];
	if (!e) return;
	const set = (value: string | number | boolean) =>
		(e.state = { value, updated_at: now(), retained: false });
	switch (cmd.command) {
		case 'turn_on':
			set(true);
			break;
		case 'turn_off':
			set(false);
			break;
		case 'set_brightness':
			e.attributes.brightness = cmd.brightness;
			set(cmd.brightness > 0);
			break;
		case 'open':
		case 'close': {
			const target = cmd.command === 'open' ? 'open' : 'closed';
			set(cmd.command === 'open' ? 'opening' : 'closing');
			setTimeout(() => set(target), 2500);
			break;
		}
		case 'stop':
			set('stopped');
			break;
	}
}
