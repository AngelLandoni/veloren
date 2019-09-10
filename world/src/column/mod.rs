use crate::{
    all::ForestKind,
    block::StructureMeta,
    generator::{Generator, SpawnRules, TownGen},
    sim::{LocationInfo, SimChunk, WorldSim},
    util::{RandomPerm, Sampler, UnitChooser},
    CONFIG,
};
use common::{
    assets,
    terrain::{BlockKind, Structure, TerrainChunkSize},
    vol::RectVolSize,
};
use lazy_static::lazy_static;
use noise::NoiseFn;
use std::{
    f32,
    ops::{Add, Div, Mul, Neg, Sub},
    sync::Arc,
};
use vek::*;

pub struct ColumnGen<'a> {
    pub sim: &'a WorldSim,
}

static UNIT_CHOOSER: UnitChooser = UnitChooser::new(0x700F4EC7);
static DUNGEON_RAND: RandomPerm = RandomPerm::new(0x42782335);

lazy_static! {
    pub static ref DUNGEONS: Vec<Arc<Structure>> = vec![
        assets::load_map("world.structure.dungeon.ruins", |s: Structure| s
            .with_center(Vec3::new(57, 58, 61))
            .with_default_kind(BlockKind::Dense))
        .unwrap(),
        assets::load_map("world.structure.dungeon.ruins_2", |s: Structure| s
            .with_center(Vec3::new(53, 57, 60))
            .with_default_kind(BlockKind::Dense))
        .unwrap(),
        assets::load_map("world.structure.dungeon.ruins_3", |s: Structure| s
            .with_center(Vec3::new(58, 45, 72))
            .with_default_kind(BlockKind::Dense))
        .unwrap(),
        assets::load_map(
            "world.structure.dungeon.meso_sewer_temple",
            |s: Structure| s
                .with_center(Vec3::new(63, 62, 60))
                .with_default_kind(BlockKind::Dense)
        )
        .unwrap(),
        assets::load_map("world.structure.dungeon.ruins_maze", |s: Structure| s
            .with_center(Vec3::new(60, 60, 116))
            .with_default_kind(BlockKind::Dense))
        .unwrap(),
    ];
}

impl<'a> ColumnGen<'a> {
    pub fn new(sim: &'a WorldSim) -> Self {
        Self { sim }
    }

    fn get_local_structure(&self, wpos: Vec2<i32>) -> Option<StructureData> {
        let (pos, seed) = self
            .sim
            .gen_ctx
            .region_gen
            .get(wpos)
            .iter()
            .copied()
            .min_by_key(|(pos, _)| pos.distance_squared(wpos))
            .unwrap();

        let chunk_pos = pos.map2(TerrainChunkSize::RECT_SIZE, |e, sz: u32| e / sz as i32);
        let chunk = self.sim.get(chunk_pos)?;

        /* let sea_level = if alt_old < CONFIG.sea_level {
            CONFIG.sea_level
        } else {
            water_level
        }; */

        if seed % 5 == 2
            && chunk.temp > CONFIG.desert_temp
            // && chunk.alt_old > CONFIG.sea_level + 5.0
            && chunk.alt > chunk.water_alt + 5.0
            && chunk.chaos <= 0.35
        {
            Some(StructureData {
                pos,
                seed,
                meta: Some(StructureMeta::Pyramid { height: 140 }),
            })
        } else if seed % 17 == 2 && chunk.chaos < 0.2 {
            Some(StructureData {
                pos,
                seed,
                meta: Some(StructureMeta::Volume {
                    units: UNIT_CHOOSER.get(seed),
                    volume: &DUNGEONS[DUNGEON_RAND.get(seed) as usize % DUNGEONS.len()],
                }),
            })
        } else {
            None
        }
    }

    fn gen_close_structures(&self, wpos: Vec2<i32>) -> [Option<StructureData>; 9] {
        let mut metas = [None; 9];
        self.sim
            .gen_ctx
            .structure_gen
            .get(wpos)
            .into_iter()
            .copied()
            .enumerate()
            .for_each(|(i, (pos, seed))| {
                metas[i] = self.get_local_structure(pos).or(Some(StructureData {
                    pos,
                    seed,
                    meta: None,
                }));
            });
        metas
    }
}

impl<'a> Sampler<'a> for ColumnGen<'a> {
    type Index = Vec2<i32>;
    type Sample = Option<ColumnSample<'a>>;

    fn get(&self, wpos: Vec2<i32>) -> Option<ColumnSample<'a>> {
        let wposf = wpos.map(|e| e as f64);
        let chunk_pos = wpos.map2(TerrainChunkSize::RECT_SIZE, |e, sz: u32| e / sz as i32);
        /* let wpos_mid = chunk_pos.map2(Vec2::from(TerrainChunkSize::RECT_SIZE), |e, sz: u32| {
            (e as u32 * sz) as i32
        });
        let wposf_mid = wpos_mid.map(|e| e as f64); */

        let sim = &self.sim;

        let turb = Vec2::new(
            sim.gen_ctx.turb_x_nz.get((wposf.div(48.0)).into_array()) as f32,
            sim.gen_ctx.turb_y_nz.get((wposf.div(48.0)).into_array()) as f32,
        ) * 12.0;
        let wposf_turb = wposf + turb.map(|e| e as f64);
        /* let turb_mid = Vec2::new(
            sim.gen_ctx.turb_x_nz.get((wposf_mid.div(48.0)).into_array()) as f32,
            sim.gen_ctx.turb_y_nz.get((wposf_mid.div(48.0)).into_array()) as f32,
        );
        let wposf_turb_mid = wposf_mid + turb_mid.map(|e| e as f64); */

        // let alt_base = sim.get_interpolated(wpos, |chunk| chunk.alt_base)?;
        let chaos = sim.get_interpolated(wpos, |chunk| chunk.chaos)?;
        // let chaos_mid = sim.get_interpolated(wpos_mid, |chunk| chunk.chaos)?;
        let temp = sim.get_interpolated(wpos, |chunk| chunk.temp)?;
        /* let downhill_alt = sim.get_interpolated(wpos, |chunk| {
            sim.get(chunk.downhill).map(|chunk| chunk.alt).unwrap_or(CONFIG.sea_level)
        })?; */
        let humidity = sim.get_interpolated(wpos, |chunk| chunk.humidity)?;
        // let humidity_mid = sim.get_interpolated(wpos_mid, |chunk| chunk.humidity)?;
        let rockiness = sim.get_interpolated(wpos, |chunk| chunk.rockiness)?;
        let tree_density = sim.get_interpolated(wpos, |chunk| chunk.tree_density)?;
        let spawn_rate = sim.get_interpolated(wpos, |chunk| chunk.spawn_rate)?;

        let sim_chunk = sim.get(chunk_pos)?;
        // let flux = sim_chunk.flux;

        // Never used
        //const RIVER_PROPORTION: f32 = 0.025;

        /*
        let river = dryness
            .abs()
            .neg()
            .add(RIVER_PROPORTION)
            .div(RIVER_PROPORTION)
            .max(0.0)
            .mul((1.0 - (chaos - 0.15) * 20.0).max(0.0).min(1.0));
        */
        let river = 0.0;

        let cliff_hill = (sim
            .gen_ctx
            .small_nz
            .get((wposf_turb.div(128.0)).into_array()) as f32)
            .mul(24.0);

        let riverless_alt_delta =
            0.0
            /*(sim
                .gen_ctx
                .small_nz
                .get((wposf_turb.div(150.0)).into_array()) as f32)
                .abs()
                .mul(chaos.max(0.025))
                .mul(64.0)
            + (sim
                .gen_ctx
                .small_nz
                .get((wposf_turb.div(450.0)).into_array()) as f32)
                .abs()
                .mul(1.0 - chaos)
                .mul(1.0 - humidity)
                .mul(96.0)*/;

        /* let riverless_alt_delta_mid =
            (sim
                .gen_ctx
                .small_nz
                .get((wposf_turb_mid.div(150.0)).into_array()) as f32)
                .abs()
                .mul(chaos_mid.max(0.025))
                .mul(64.0)
            + (sim
                .gen_ctx
                .small_nz
                .get((wposf_turb_mid.div(450.0)).into_array()) as f32)
                .abs()
                .mul(1.0 - chaos_mid)
                .mul(1.0 - humidity_mid)
                .mul(96.0); */

        let riverless_alt_delta = riverless_alt_delta
            - (1.0 - river)
                .mul(f32::consts::PI)
                .cos()
                .add(1.0)
                .mul(0.5)
                .mul(24.0);

        // let alt_orig = sim_chunk.alt;
        let downhill = sim_chunk.downhill;
        let downhill_pos = downhill.and_then(|downhill_pos| sim.get(downhill_pos));
        /* let downhill_alt = downhill_pos
            .map(|downhill_chunk| downhill_chunk.alt)
            .unwrap_or(CONFIG.sea_level); */
        let flux = sim.get_interpolated(wpos, |chunk| chunk.flux)?;
        // let flux = sim_chunk.flux;
        let downhill_flux = downhill_pos
            .map(|downhill_chunk| downhill_chunk.flux)
            .unwrap_or(flux);
        /* let downhill_pos_x = sim.get_interpolated(wpos, |chunk| {
            chunk.downhill.map(|e|
                e.map2(Vec2::from(TerrainChunkSize::RECT_SIZE), |e, sz: u32| e as f32 / sz as f32)
            ).unwrap_or(wpos.map(|e| e as f32))
                .x
        })?;
        let downhill_pos_y = sim.get_interpolated(wpos, |chunk| {
            chunk.downhill.map(|e|
                e.map2(Vec2::from(TerrainChunkSize::RECT_SIZE), |e, sz: u32| e as f32 / sz as f32)
            ).unwrap_or(wpos.map(|e| e as f32))
                .y
        })?;
        let downhill_pos = Vec2::new(downhill_pos_x, downhill_pos_y).map(|e| e as i32);
        let flux = sim.get_interpolated(wpos, |chunk| chunk.flux)?;
        let downhill_flux = sim.get_interpolated(downhill_pos, |chunk| chunk.flux)?; */
        /* let downhill_flux = sim.get_interpolated(wpos, |chunk| {
            let downhill = chunk.downhill;
            let downhill_pos = downhill.and_then(|downhill_pos| sim.get(downhill_pos));
            let downhill_alt = downhill_pos.map(|downhill_chunk| downhill_chunk.alt)
                .unwrap_or(CONFIG.sea_level);
            // let flux = sim.get_interpolated(chunk., |chunk| chunk.flux)?;
            // let flux = chunk.flux;
            let downhill_flux = downhill_pos
                .map(|downhill_chunk| downhill_chunk.flux)
                .unwrap_or(flux);
            downhill_flux
            /* Lerp::lerp(
                downhill_flux,
                flux,
                (wpos - downhill.unwrap_or(wpos))
                    .map2(Vec2::from(TerrainChunkSize::RECT_SIZE), |e, sz: u32| {
                        e as f32 / sz as f32
                    })
                    // TODO: Make from 0 to 1 on diagonals.
                    .magnitude(),
            ) */
        })?; */
        let flux =
            Lerp::lerp(
                downhill_flux,
                flux,
                (wpos - downhill.unwrap_or(wpos)/*downhill_pos*/)
                    .map2(Vec2::from(TerrainChunkSize::RECT_SIZE), |e, sz: u32| {
                        e as f32 / sz as f32
                    })
                    // TODO: Make from 0 to 1 on diagonals.
                    .magnitude(),
            );

                /*Lerp::lerp((alt - 16.0).min(alt), alt.min(alt), (1.0 - flux/*(flux - 0.85) / (1.0 - 0.85)*/ * water_factor)),
                (flux * water_factor - /*0.33*/0.25) * 256.0,
                    ),*/

        let water_factor = (1024.0 * 1024.0) / 50.0;
        let alt = sim.get_interpolated(wpos, |chunk| {
            chunk.alt
            /* let new_alt = Lerp::lerp(chunk.alt - 5.0, chunk.alt, chunk.flux * water_factor);
            let new_water_alt = chunk.water_alt.max(new_alt);
            Lerp::lerp(
                chunk.alt - 5.0,
                new_alt,
                chunk.alt - new_water_alt,
            ) */
        })? + riverless_alt_delta;
        let alt_old = sim.get_interpolated(wpos, |chunk| chunk.alt_old)? + riverless_alt_delta;
        // let alt_mid = sim.get_interpolated(wpos_mid, |chunk| chunk.alt)?;
        let water_alt_orig = sim_chunk.water_alt;// + riverless_alt_delta;
        let water_alt = sim.get_interpolated(wpos, |chunk| {
            chunk.water_alt
                .max(/*Lerp::lerp(chunk.alt - 5.0, chunk.alt, chunk.flux * water_factor)*/chunk.alt - 5.0)
        /*    Lerp::lerp(
                water_alt/*./*max(alt_orig).*/max(alt - 5.0)*/,
                /*alt - 5.0*/
                alt,
                (flux/*(flux - 0.85) / (1.0 - 0.85)*/ * water_factor)
            );
        }*/

        })?;// + riverless_alt_delta;
        // let water_alt = sim_chunk.water_alt;// + riverless_alt_delta;

        let is_cliffs = sim_chunk.is_cliffs;
        let near_cliffs = sim_chunk.near_cliffs;

        // Logistic regression.  Make sure x ∈ (0, 1).
        // let logit = |x: f32| x.ln() - x.neg().ln_1p();
        // 0.5 + 0.5 * tanh(ln(1 / (1 - 0.1) - 1) / (2 * (sqrt(3)/pi)))
        // let logistic_2_base = 3.0f32.sqrt().mul(f32::consts::FRAC_2_PI);
        // Assumes μ = 0, σ = 1
        // let logistic_cdf = |x: f32| x.div(logistic_2_base).tanh().mul(0.5).add(0.5);

        /* let water_level =
            riverless_alt - 4.0 /*- 5.0 * chaos */+ logistic_cdf(logit(flux)) * 8.0;
            /*if flux > 0.98 {
            riverless_alt
        } else {
            riverless_alt - 4.0 - 5.0 * chaos
        }; */ */
        // let water_level = riverless_alt - 4.0 - 5.0 * chaos;
        // let water_level = water_alt;
        let (alt, water_level) = /*if /*water_alt_orig == alt_orig.max(0.0)*/water_alt_orig == CONFIG.sea_level {
            // This is flowing into the ocean.
            if alt_orig <= CONFIG.sea_level + 5.0 {
                (alt, CONFIG.sea_level)
            } else {
                (
                    Lerp::lerp(
                        alt,
                        Lerp::lerp((alt - 16.0).min(alt), alt.min(alt), (1.0 - flux/*(flux - 0.85) / (1.0 - 0.85)*/ * water_factor)),
                        (flux * water_factor - /*0.33*/0.25) * 256.0,
                    ),
                    /*Lerp::lerp(
                       alt,*/
                       // Lerp::lerp(alt.min(water_alt), water_alt, flux * water_factor * 16.0),
                       Lerp::lerp(alt - 16.0, alt, (/*(flux - 0.85) / (1.0 - 0.85)*/flux) * water_factor),
                       /*(flux - water_factor * 0.33) * 256.0,*/
                )
            }
        } else *//*if let Some(downhill) = downhill */{
            // This is flowing into a lake, or a lake, or is at least a non-ocean tile.
            //
            // If we are <= water_alt, we are in the lake; otherwise, we are flowing into it.
            /* if alt <= water_alt {
                (alt - 5.0, water_alt)
            } else */{
                /* // Compute the delta between alt and the "real" height of the chunk.
                let z_delta = alt - alt_orig;
                // Also find the horizontal delta.
                let xy_delta = wposf - wposf_mid; */
                // Find the slope, extend it to distance 5, and then project to the z axis.
                let new_water_alt =
                        Lerp::lerp(
                            water_alt/*./*max(alt_orig).*/max(alt - 5.0)*/,
                            /*alt - 5.0*/
                            alt,
                            flux/*(flux - 0.85) / (1.0 - 0.85)*/ * water_factor
                        );
                let new_alt =
                    Lerp::lerp(
                        alt,
                        alt - 5.0,
                        flux/*(flux - 0.85) / (1.0 - 0.85)*/ * water_factor
                    );
                (
                    Lerp::lerp(
                        alt - 5.0,
                        new_alt,
                        (alt - /*5.0 - */water_alt/*.max(alt_orig)*/)/*.div(CONFIG.mountain_scale * 0.25)*/ * 1.0,
                    ),
                    Lerp::lerp(
                        /*water_alt_orig*/water_alt.max(water_alt_orig)/*./*max(alt_orig).*/max(alt - 5.0)*/,
                        new_water_alt,
                        (alt /*- 5.0 */- water_alt/*.max(alt_orig)*/) * 1.0,
                    ),
                )
                /*(
                    Lerp::lerp(
                        alt,
                        Lerp::lerp(water_alt, alt, (1.0 - flux/*(flux - 0.85) / (1.0 - 0.85)*/ * water_factor)),
                        (alt - water_alt) * 256.0 + (flux * water_factor - /*0.33*/0.25) * 256.0,
                    ),
                    /*Lerp::lerp(
                       alt,*/
                       // Lerp::lerp(alt.min(water_alt), water_alt, flux * water_factor * 16.0),
                       Lerp::lerp(water_alt, alt, (/*(flux - 0.85) / (1.0 - 0.85)*/flux) * water_factor),
                       /*(flux - water_factor * 0.33) * 256.0,*/
                )*/
            }
        }/* else {
            // Ocean tile, just use sea level.
            (alt, water_alt_orig)
        }*/;

        // let water_factor_diff = water_factor / 2.0;
        // let water_level = water_alt;
        // water_level is not the minimum, the minimum is lake bottom.  Have the water dry up
        // towards the lake bottom.
        // let water_level = water_alt;// - 1.6 - 4.0 - 5.0 * chaos;
        // let water_level = Lerp::lerp(alt.min(water_alt), water_alt, flux * water_factor * 16.0);// - 1.6 - 4.0 - 5.0 * chaos;
        // let water_level = Lerp::lerp((alt - riverless_alt_delta).min(water_alt), water_alt, flux * water_factor * 16.0);

        /*} else {
            (alt, CONFIG.sea_level)
            // riverless_alt - 4.0 - 5.0 * chaos
        }*/;

        let rock = (sim.gen_ctx.small_nz.get(
            Vec3::new(wposf.x, wposf.y, alt as f64)
                .div(100.0)
                .into_array(),
        ) as f32)
            .mul(rockiness)
            .sub(0.4)
            .max(0.0)
            .mul(8.0);

        let wposf3d = Vec3::new(wposf.x, wposf.y, alt as f64);

        let marble_small = (sim.gen_ctx.hill_nz.get((wposf3d.div(3.0)).into_array()) as f32)
            .add(1.0)
            .mul(0.5);
        let marble = (sim.gen_ctx.hill_nz.get((wposf3d.div(48.0)).into_array()) as f32)
            .mul(0.75)
            .add(1.0)
            .mul(0.5)
            .add(marble_small.sub(0.5).mul(0.25));

        let temp = temp.add((marble - 0.5) * 0.25);
        let humidity = humidity.add((marble - 0.5) * 0.25);

        // Colours
        let cold_grass = Rgb::new(0.0, 0.5, 0.25);
        let warm_grass = Rgb::new(0.03, 0.8, 0.0);
        let dark_grass = Rgb::new(0.01, 0.3, 0.0);
        let wet_grass = Rgb::new(0.1, 0.8, 0.2);
        let cold_stone = Rgb::new(0.57, 0.67, 0.8);
        let warm_stone = Rgb::new(0.77, 0.77, 0.64);
        let beach_sand = Rgb::new(0.89, 0.87, 0.64);
        let desert_sand = Rgb::new(0.93, 0.80, 0.54);
        let snow = Rgb::new(0.8, 0.85, 1.0);

        let dirt = Lerp::lerp(
            Rgb::new(0.078, 0.078, 0.20),
            Rgb::new(0.61, 0.49, 0.0),
            marble,
        );
        let tundra = Lerp::lerp(snow, Rgb::new(0.01, 0.3, 0.0), 0.4 + marble * 0.6);
        let dead_tundra = Lerp::lerp(warm_stone, Rgb::new(0.3, 0.12, 0.2), marble);
        let cliff = Rgb::lerp(cold_stone, warm_stone, marble);

        let grass = Rgb::lerp(
            cold_grass,
            warm_grass,
            marble.sub(0.5).add(1.0.sub(humidity).mul(0.5)).powf(1.5),
        );
        let snow_moss = Rgb::lerp(snow, cold_grass, 0.4 + marble.powf(1.5) * 0.6);
        let moss = Rgb::lerp(dark_grass, cold_grass, marble.powf(1.5));
        let rainforest = Rgb::lerp(wet_grass, warm_grass, marble.powf(1.5));
        let sand = Rgb::lerp(beach_sand, desert_sand, marble);

        let tropical = Rgb::lerp(
            Rgb::lerp(
                grass,
                Rgb::new(0.15, 0.2, 0.15),
                marble_small
                    .sub(0.5)
                    .mul(0.2)
                    .add(0.75.mul(1.0.sub(humidity)))
                    .powf(0.667),
            ),
            Rgb::new(0.87, 0.62, 0.56),
            marble.powf(1.5).sub(0.5).mul(4.0),
        );

        // For below desert humidity, we are always sand or rock, depending on altitude and
        // temperature.
        let ground = Rgb::lerp(
            Rgb::lerp(
                dead_tundra,
                sand,
                temp.sub(CONFIG.snow_temp)
                    .div(CONFIG.desert_temp.sub(CONFIG.snow_temp))
                    .mul(0.5),
            ),
            cliff,
            alt.sub(CONFIG.mountain_scale * 0.25)
                .div(CONFIG.mountain_scale * 0.125),
        );
        // From desert to forest humidity, we go from tundra to dirt to grass to moss to sand,
        // depending on temperature.
        let ground = Rgb::lerp(
            ground,
            Rgb::lerp(
                Rgb::lerp(
                    Rgb::lerp(
                        Rgb::lerp(
                            tundra,
                            // snow_temp to 0
                            dirt,
                            temp.sub(CONFIG.snow_temp)
                                .div(CONFIG.snow_temp.neg())
                                /*.sub((marble - 0.5) * 0.05)
                                .mul(256.0)*/
                                .mul(1.0),
                        ),
                        // 0 to tropical_temp
                        grass,
                        temp.div(CONFIG.tropical_temp).mul(4.0),
                    ),
                    // tropical_temp to desert_temp
                    moss,
                    temp.sub(CONFIG.tropical_temp)
                        .div(CONFIG.desert_temp.sub(CONFIG.tropical_temp))
                        .mul(1.0),
                ),
                // above desert_temp
                sand,
                temp.sub(CONFIG.desert_temp)
                    .div(1.0 - CONFIG.desert_temp)
                    .mul(4.0),
            ),
            humidity
                .sub(CONFIG.desert_hum)
                .div(CONFIG.forest_hum.sub(CONFIG.desert_hum))
                .mul(1.0),
        );
        // From forest to jungle humidity, we go from snow to dark grass to grass to tropics to sand
        // depending on temperature.
        let ground = Rgb::lerp(
            ground,
            Rgb::lerp(
                Rgb::lerp(
                    Rgb::lerp(
                        snow_moss,
                        // 0 to tropical_temp
                        grass,
                        temp.div(CONFIG.tropical_temp).mul(4.0),
                    ),
                    // tropical_temp to desert_temp
                    tropical,
                    temp.sub(CONFIG.tropical_temp)
                        .div(CONFIG.desert_temp.sub(CONFIG.tropical_temp))
                        .mul(1.0),
                ),
                // above desert_temp
                sand,
                temp.sub(CONFIG.desert_temp)
                    .div(1.0 - CONFIG.desert_temp)
                    .mul(4.0),
            ),
            humidity
                .sub(CONFIG.forest_hum)
                .div(CONFIG.jungle_hum.sub(CONFIG.forest_hum))
                .mul(1.0),
        );
        // From jungle humidity upwards, we go from snow to grass to rainforest to tropics to sand.
        let ground = Rgb::lerp(
            ground,
            Rgb::lerp(
                Rgb::lerp(
                    Rgb::lerp(
                        snow_moss,
                        // 0 to tropical_temp
                        rainforest,
                        temp.div(CONFIG.tropical_temp).mul(4.0),
                    ),
                    // tropical_temp to desert_temp
                    tropical,
                    temp.sub(CONFIG.tropical_temp)
                        .div(CONFIG.desert_temp.sub(CONFIG.tropical_temp))
                        .mul(4.0),
                ),
                // above desert_temp
                sand,
                temp.sub(CONFIG.desert_temp)
                    .div(1.0 - CONFIG.desert_temp)
                    .mul(4.0),
            ),
            humidity.sub(CONFIG.jungle_hum).mul(1.0),
        );

        // Snow covering
        let ground = Rgb::lerp(
            snow,
            ground,
            temp.sub(CONFIG.snow_temp)
                .max(-humidity.sub(CONFIG.desert_hum))
                .mul(16.0)
                .add((marble_small - 0.5) * 0.5),
        );

        /*
        // Work out if we're on a path or near a town
        let dist_to_path = match &sim_chunk.location {
            Some(loc) => {
                let this_loc = &sim.locations[loc.loc_idx];
                this_loc
                    .neighbours
                    .iter()
                    .map(|j| {
                        let other_loc = &sim.locations[*j];

                        // Find the two location centers
                        let near_0 = this_loc.center.map(|e| e as f32);
                        let near_1 = other_loc.center.map(|e| e as f32);

                        // Calculate distance to path between them
                        (0.0 + (near_1.y - near_0.y) * wposf_turb.x as f32
                            - (near_1.x - near_0.x) * wposf_turb.y as f32
                            + near_1.x * near_0.y
                            - near_0.x * near_1.y)
                            .abs()
                            .div(near_0.distance(near_1))
                    })
                    .filter(|x| x.is_finite())
                    .min_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap_or(f32::INFINITY)
            }
            None => f32::INFINITY,
        };

        let on_path = dist_to_path < 5.0 && !sim_chunk.near_cliffs; // || near_0.distance(wposf_turb.map(|e| e as f32)) < 150.0;

        let (alt, ground) = if on_path {
            (alt - 1.0, dirt)
        } else {
            (alt, ground)
        };
        */

        // Cities
        // TODO: In a later MR
        /*
        let building = match &sim_chunk.location {
            Some(loc) => {
                let loc = &sim.locations[loc.loc_idx];
                let rpos = wposf.map2(loc.center, |a, b| a as f32 - b as f32) / 256.0 + 0.5;

                if rpos.map(|e| e >= 0.0 && e < 1.0).reduce_and() {
                    (loc.settlement
                        .get_at(rpos)
                        .map(|b| b.seed % 20 + 10)
                        .unwrap_or(0)) as f32
                } else {
                    0.0
                }
            }
            None => 0.0,
        };

        let alt = alt + building;
        */

        // Caves
        let cave_at = |wposf: Vec2<f64>| {
            (sim.gen_ctx.cave_0_nz.get(
                Vec3::new(wposf.x, wposf.y, alt as f64 * 8.0)
                    .div(800.0)
                    .into_array(),
            ) as f32)
                .powf(2.0)
                .neg()
                .add(1.0)
                .mul((1.15 - chaos).min(1.0))
        };
        let cave_xy = cave_at(wposf);
        let cave_alt = alt - 24.0
            + (sim
                .gen_ctx
                .cave_1_nz
                .get(Vec2::new(wposf.x, wposf.y).div(48.0).into_array()) as f32)
                * 8.0
            + (sim
                .gen_ctx
                .cave_1_nz
                .get(Vec2::new(wposf.x, wposf.y).div(500.0).into_array()) as f32)
                .add(1.0)
                .mul(0.5)
                .powf(15.0)
                .mul(150.0);

        /* let sea_level = if alt_old < CONFIG.sea_level {
            CONFIG.sea_level
        } else {
            water_level
        }; */

        Some(ColumnSample {
            alt,
            alt_old,
            chaos,
            sea_level: water_alt_orig,
            water_level,
            river,
            surface_color: Rgb::lerp(
                sand,
                // Land
                /*Rgb::lerp(
                    ground,
                    // Mountain
                    Rgb::lerp(
                        cliff,
                        snow,
                        (alt - CONFIG.sea_level
                            - 0.7 * CONFIG.mountain_scale
                            // - alt_base
                            - temp * 96.0
                            - marble * 24.0)
                            / 12.0,
                    ),
                    (alt - CONFIG.sea_level - 0.25 * CONFIG.mountain_scale + marble * 128.0)
                        / (0.25 * CONFIG.mountain_scale),
                ),*/
                ground,
                // Beach
                ((alt_old - CONFIG.sea_level/*alt - sea_level*/ - 1.0) / 2.0)
                    .min(1.0 - river * 2.0)
                    .max(0.0),
            ),
            sub_surface_color: dirt,
            tree_density,
            forest_kind: sim_chunk.forest_kind,
            close_structures: self.gen_close_structures(wpos),
            cave_xy,
            cave_alt,
            marble,
            marble_small,
            rock,
            is_cliffs,
            near_cliffs,
            cliff_hill,
            close_cliffs: sim.gen_ctx.cliff_gen.get(wpos),
            temp,
            spawn_rate,
            location: sim_chunk.location.as_ref(),

            chunk: sim_chunk,
            spawn_rules: sim_chunk
                .structures
                .town
                .as_ref()
                .map(|town| TownGen.spawn_rules(town, wpos))
                .unwrap_or(SpawnRules::default()),
        })
    }
}

#[derive(Clone)]
pub struct ColumnSample<'a> {
    pub alt: f32,
    pub alt_old: f32,
    pub chaos: f32,
    pub sea_level: f32,
    pub water_level: f32,
    pub river: f32,
    pub surface_color: Rgb<f32>,
    pub sub_surface_color: Rgb<f32>,
    pub tree_density: f32,
    pub forest_kind: ForestKind,
    pub close_structures: [Option<StructureData>; 9],
    pub cave_xy: f32,
    pub cave_alt: f32,
    pub marble: f32,
    pub marble_small: f32,
    pub rock: f32,
    pub is_cliffs: bool,
    pub near_cliffs: bool,
    pub cliff_hill: f32,
    pub close_cliffs: [(Vec2<i32>, u32); 9],
    pub temp: f32,
    pub spawn_rate: f32,
    pub location: Option<&'a LocationInfo>,

    pub chunk: &'a SimChunk,
    pub spawn_rules: SpawnRules,
}

#[derive(Copy, Clone)]
pub struct StructureData {
    pub pos: Vec2<i32>,
    pub seed: u32,
    pub meta: Option<StructureMeta>,
}
