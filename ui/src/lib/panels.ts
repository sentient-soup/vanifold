// Domain layer for the dashboard: taxonomy v2 panels, featured-metric
// heuristics, and alert thresholds. See docs/ui-taxonomy.md for the analysis
// these rules come from. Heuristics pick sane heroes with zero config and
// degrade honestly when a metric is missing.

import type { Entity } from './types';
import { kindName, quality } from './types';

export type PanelKey =
	| 'power'
	| 'water'
	| 'climate'
	| 'lighting'
	| 'access'
	| 'vehicle'
	| 'media'
	| 'misc';

export const PANELS: { key: PanelKey; label: string }[] = [
	{ key: 'power', label: 'Power' },
	{ key: 'water', label: 'Water' },
	{ key: 'climate', label: 'Climate' },
	{ key: 'lighting', label: 'Lighting' },
	{ key: 'access', label: 'Access' },
	{ key: 'vehicle', label: 'Vehicle' },
	{ key: 'media', label: 'Media' },
	{ key: 'misc', label: 'Other' }
];

/** The six panels always shown; media/misc appear only when populated. */
export const CORE_PANELS: PanelKey[] = ['power', 'water', 'climate', 'lighting', 'access', 'vehicle'];

export function isPanelKey(k: string): k is PanelKey {
	return PANELS.some((p) => p.key === k);
}

export function ofSubsystem(entities: Record<string, Entity>, key: PanelKey): Entity[] {
	return Object.values(entities)
		.filter((e) => (e.subsystem ?? 'misc') === key)
		.sort((a, b) => a.device_id.localeCompare(b.device_id) || a.name.localeCompare(b.name));
}

export function num(e: Entity | null | undefined): number | null {
	const v = e?.state?.value;
	return typeof v === 'number' ? v : null;
}

export function fmt(v: number | null, digits = 0): string {
	if (v === null) return '—';
	return Number.isInteger(v) && digits === 0 ? String(v) : v.toFixed(digits);
}

const byClass = (list: Entity[], cls: string) => list.filter((e) => e.device_class === cls);
const nameHas = (e: Entity, hints: string[]) =>
	hints.some((h) => e.name.toLowerCase().includes(h));
const isKind = (e: Entity, k: string) => kindName(e.kind) === k;

// ---------- Featured-metric heroes ----------

export interface PowerHero {
	soc: Entity | null;
	voltage: Entity | null;
	batteryTemp: Entity | null;
	solar: Entity | null;
	wattsTotal: number | null;
	used: Set<string>;
}

export function powerHero(list: Entity[]): PowerHero {
	const soc = byClass(list, 'battery').find((e) => e.unit === '%') ?? null;
	const voltages = byClass(list, 'voltage');
	const voltage =
		(soc && voltages.find((e) => e.device_id === soc.device_id)) ??
		voltages.find((e) => nameHas(e, ['battery', 'bus'])) ??
		voltages[0] ??
		null;
	const batteryTemp =
		list.find((e) => e.device_class === 'temperature' && nameHas(e, ['battery'])) ?? null;
	const powers = byClass(list, 'power');
	const solar = powers.find((e) => nameHas(e, ['solar'])) ?? null;
	const live = powers.map(num).filter((v): v is number => v !== null);
	const wattsTotal = live.length ? live.reduce((s, v) => s + v, 0) : null;
	return {
		soc,
		voltage,
		batteryTemp,
		solar,
		wattsTotal,
		used: new Set([soc, voltage, batteryTemp].filter(Boolean).map((e) => e!.id))
	};
}

export interface Tank {
	entity: Entity;
	role: 'fresh' | 'grey' | 'other';
}

export interface WaterHero {
	tanks: Tank[];
	pump: Entity | null;
	used: Set<string>;
}

export function waterHero(list: Entity[]): WaterHero {
	const tanks: Tank[] = list
		.filter((e) => isKind(e, 'sensor') && e.unit === '%')
		.map((e) => ({
			entity: e,
			role: nameHas(e, ['grey', 'gray', 'waste', 'black'])
				? ('grey' as const)
				: nameHas(e, ['fresh', 'water', 'tank'])
					? ('fresh' as const)
					: ('other' as const)
		}))
		.sort((a, b) => (a.role === 'fresh' ? -1 : 1) - (b.role === 'fresh' ? -1 : 1));
	const pump = list.find((e) => isKind(e, 'switch') && nameHas(e, ['pump'])) ??
		list.find((e) => isKind(e, 'switch')) ?? null;
	return {
		tanks,
		pump,
		used: new Set([...tanks.map((t) => t.entity.id), ...(pump ? [pump.id] : [])])
	};
}

export interface ClimateHero {
	inside: Entity | null;
	humidity: Entity | null;
	used: Set<string>;
}

export function climateHero(list: Entity[]): ClimateHero {
	const temps = byClass(list, 'temperature');
	const inside =
		temps.find((e) => nameHas(e, ['inside', 'cabin', 'interior', 'indoor'])) ?? temps[0] ?? null;
	const humidity = byClass(list, 'humidity')[0] ?? null;
	return {
		inside,
		humidity,
		used: new Set([inside, humidity].filter(Boolean).map((e) => e!.id))
	};
}

export interface AccessSummary {
	movers: Entity[]; // covers: rear entry, bed, awning
	openings: Entity[]; // binary door/window contacts
	openCount: number;
	unknownCount: number;
	used: Set<string>;
}

export function accessSummary(list: Entity[]): AccessSummary {
	const movers = list.filter((e) => isKind(e, 'cover'));
	const openings = list.filter(
		(e) =>
			isKind(e, 'binary_sensor') &&
			(['door', 'window', 'garage_door', 'opening', 'lock'].includes(e.device_class ?? '') ||
				nameHas(e, ['door', 'window']))
	);
	// HA binary door semantics: true = open.
	const openCount = openings.filter((e) => e.state?.value === true).length;
	const unknownCount = openings.filter((e) => !e.state).length;
	return {
		movers,
		openings,
		openCount,
		unknownCount,
		used: new Set([...movers, ...openings].map((e) => e.id))
	};
}

export interface VehicleHero {
	fuel: Entity | null;
	starter: Entity | null;
	used: Set<string>;
}

export function vehicleHero(list: Entity[]): VehicleHero {
	const fuel =
		list.find((e) => e.unit === '%' && nameHas(e, ['fuel'])) ??
		list.find((e) => e.unit === '%') ??
		null;
	const starter = byClass(list, 'voltage')[0] ?? null;
	return { fuel, starter, used: new Set([fuel, starter].filter(Boolean).map((e) => e!.id)) };
}

export function lights(list: Entity[]): Entity[] {
	return list.filter((e) => isKind(e, 'light'));
}

// ---------- Alerts (docs/ui-taxonomy.md thresholds; config later) ----------

export interface Alert {
	level: 'warn' | 'crit';
	text: string;
	panel: PanelKey;
}

export function computeAlerts(entities: Record<string, Entity>, now: number): Alert[] {
	const out: Alert[] = [];
	const power = ofSubsystem(entities, 'power');
	const water = ofSubsystem(entities, 'water');

	const soc = num(powerHero(power).soc);
	if (soc !== null && soc < 10) out.push({ level: 'crit', text: `Battery ${fmt(soc)}%`, panel: 'power' });
	else if (soc !== null && soc < 20) out.push({ level: 'warn', text: `Battery ${fmt(soc)}%`, panel: 'power' });

	const temp = num(powerHero(power).batteryTemp);
	if (temp !== null && temp > 45) out.push({ level: 'warn', text: `Battery ${fmt(temp, 1)}°`, panel: 'power' });

	for (const t of waterHero(water).tanks) {
		const v = num(t.entity);
		if (v === null) continue;
		if (t.role === 'grey' && v > 85) out.push({ level: 'warn', text: `Grey ${fmt(v)}%`, panel: 'water' });
		if (t.role === 'fresh' && v < 15) out.push({ level: 'warn', text: `Fresh ${fmt(v)}%`, panel: 'water' });
	}

	for (const e of Object.values(entities)) {
		if (e.criticality === 'safety' && quality(e, now) === 'unavailable') {
			out.push({
				level: 'warn',
				text: `${e.name} offline`,
				panel: (e.subsystem as PanelKey) ?? 'misc'
			});
		}
	}
	return out.sort((a, b) => (a.level === 'crit' ? -1 : 1) - (b.level === 'crit' ? -1 : 1));
}

/** Gauge tone from an alert-style reading. */
export function levelTone(v: number | null, opts: { warnBelow?: number; critBelow?: number; warnAbove?: number }): 'ok' | 'warn' | 'crit' {
	if (v === null) return 'ok';
	if (opts.critBelow !== undefined && v < opts.critBelow) return 'crit';
	if (opts.warnBelow !== undefined && v < opts.warnBelow) return 'warn';
	if (opts.warnAbove !== undefined && v > opts.warnAbove) return 'warn';
	return 'ok';
}
