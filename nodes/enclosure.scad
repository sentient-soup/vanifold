// vanifold node shell: parametric box + screw-down lid.
// Print in PETG or ASA, never PLA: a parked van's interior passes PLA's
// ~55C softening point in summer.
//
// Cables leave through PG7 cable glands as short pigtails ending in Deutsch
// connectors (docs/prototype.md), so nothing here depends on connector
// panel cutouts.
//
// Export (defaults = rear-entry node):
//   openscad -D 'part="base"' -o rear-entry-base.stl enclosure.scad
//   openscad -D 'part="lid"'  -o rear-entry-lid.stl  enclosure.scad
// Solar-tilt node (bigger, four glands):
//   openscad -D 'part="base"' -D 'inner=[180,120,45]' \
//     -D 'glands=[[0,40,22],[0,80,22],[0,120,22],[0,160,22]]' \
//     -o solar-tilt-base.stl enclosure.scad

part = "both";            // "base", "lid", or "both" (preview)
inner = [140, 90, 45];    // usable space, mm; corner posts eat ~8mm per corner
wall = 2.4;
floor_t = 2.4;
lid_t = 2.4;
r = 4;                    // outer corner radius

post_d = 8;
post_hole = 2.5;          // 2.5 = M3 self-tapping; 4.0 = M3 heat-set insert
lid_hole = 3.4;           // M3 clearance

// Cable glands: [wall, along, up]
//   wall: 0 front (-y), 1 right (+x), 2 back (+y), 3 left (-x)
//   along: mm along the wall's axis (x for front/back, y for left/right)
//   up: mm above the inner floor (keep >= 12 so the gland nut clears it)
gland_d = 12.5;           // PG7; PG9 = 15.2
glands = [[0, 35, 22], [0, 70, 22], [0, 105, 22]];

// Board standoffs: [x, y] hole centers measured from the inner front-left
// corner. Empty = mount boards with VHB tape or add holes after measuring.
standoffs = [];
standoff_h = 6;
standoff_d = 6;

ear_w = 16;               // mounting ears on the short sides
ear_t = 4;
ear_hole = 4.5;           // M4 / #8 screw

$fn = 48;
outer = [inner.x + 2 * wall, inner.y + 2 * wall, inner.z + floor_t];
inset = wall + post_d / 2 - 1;  // posts overlap the walls by 1mm
posts = [for (x = [inset, outer.x - inset], y = [inset, outer.y - inset]) [x, y]];

module rbox(size, rad) {
    hull() for (x = [rad, size.x - rad], y = [rad, size.y - rad])
        translate([x, y, 0]) cylinder(r = rad, h = size.z);
}

module gland_hole(g) {
    z = floor_t + g[2];
    if (g[0] == 0) translate([g[1], wall / 2, z]) rotate([90, 0, 0]) cylinder(d = gland_d, h = wall + 2, center = true);
    if (g[0] == 2) translate([g[1], outer.y - wall / 2, z]) rotate([90, 0, 0]) cylinder(d = gland_d, h = wall + 2, center = true);
    if (g[0] == 1) translate([outer.x - wall / 2, g[1], z]) rotate([0, 90, 0]) cylinder(d = gland_d, h = wall + 2, center = true);
    if (g[0] == 3) translate([wall / 2, g[1], z]) rotate([0, 90, 0]) cylinder(d = gland_d, h = wall + 2, center = true);
}

module base() {
    difference() {
        union() {
            difference() {
                rbox(outer, r);
                translate([wall, wall, floor_t]) rbox([inner.x, inner.y, inner.z + 1], max(r - wall, 0.5));
            }
            for (p = posts) translate([p.x, p.y, 0]) cylinder(d = post_d, h = outer.z);
            for (s = standoffs) translate([wall + s.x, wall + s.y, 0]) cylinder(d = standoff_d, h = floor_t + standoff_h);
            translate([-ear_w, outer.y / 2 - ear_w, 0]) cube([ear_w + wall, 2 * ear_w, ear_t]);
            translate([outer.x - wall, outer.y / 2 - ear_w, 0]) cube([ear_w + wall, 2 * ear_w, ear_t]);
        }
        for (p = posts) translate([p.x, p.y, floor_t]) cylinder(d = post_hole, h = outer.z);
        for (s = standoffs) translate([wall + s.x, wall + s.y, floor_t]) cylinder(d = 2.5, h = standoff_h + 1);
        for (g = glands) gland_hole(g);
        for (x = [-ear_w / 2, outer.x + ear_w / 2]) translate([x, outer.y / 2, -1]) cylinder(d = ear_hole, h = ear_t + 2);
    }
}

module lid() {
    difference() {
        rbox([outer.x, outer.y, lid_t], r);
        for (p = posts) translate([p.x, p.y, -1]) cylinder(d = lid_hole, h = lid_t + 2);
    }
}

if (part == "base" || part == "both") base();
if (part == "lid") lid();
if (part == "both") translate([0, 0, outer.z + 10]) lid();
