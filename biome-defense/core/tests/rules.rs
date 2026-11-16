//! Tests of the rules: the maps, the waves, the points, the save format,
//! and whole stages played by the stand-in player and by a random one.

use biome_core::bot::{expected_profile, play, Style};
use biome_core::campaign::{Profile, FOLLOWERS};
use biome_core::creature::{branch_cost, muster, Cr, FRIENDS, HEROES};
use biome_core::defs::{Foe, Tw, BIOMES, FOE_COUNT, TOWERS};
use biome_core::level::{is_finale, waves, Difficulty, LEVELS, LEVEL_COUNT, PER_BIOME};
use biome_core::sim::{Phase, Setup, Sim};
use biome_core::{v2, Rng};

fn setup(level: usize, hero: Cr, difficulty: Difficulty, seed: u64) -> Setup {
    let p = expected_profile(level, hero);
    Setup { level, difficulty, pack: p.pack(), guests: p.guests(level), seed }
}

// ---- maps and waves ---------------------------------------------------------

#[test]
fn every_map_is_playable() {
    for (i, def) in LEVELS.iter().enumerate() {
        let map = def.map();
        assert!(!map.routes.is_empty(), "stage {i} has no route");
        assert!(map.mounds.len() >= 6, "stage {i} has only {} mounds", map.mounds.len());
        for r in &map.routes {
            assert!(r.len > 12.0 && r.len < 50.0, "stage {i}: a route is {} tiles long", r.len);
            let end = r.at(r.len).0;
            assert!(end.dist(map.home) < 0.01, "stage {i}: a route does not end at the home");
        }
        // a mound that cannot see the trail is a trap for the player
        for (m, &pos) in map.mounds.iter().enumerate() {
            let off = map.nearest_trail(pos).2;
            assert!(off < 1.5, "stage {i}: mound {m} is {off} tiles from the trail");
        }
        assert_eq!(def.biome, i / PER_BIOME);
    }
}

#[test]
fn waves_are_what_the_stage_promises() {
    for (i, def) in LEVELS.iter().enumerate() {
        let ws = waves(i);
        assert_eq!(ws.len(), def.waves as usize);
        assert!(def.crowd.len() == ws.len(), "stage {i} has not been tuned: run tools/apply_tune.py");
        let pot: f32 = ws.iter().map(|w| w.pot).sum();
        assert!((pot - 1.0).abs() < 1e-3);
        for (w, wave) in ws.iter().enumerate() {
            assert!(!wave.groups.is_empty());
            for g in &wave.groups {
                assert!(g.count >= 1 && g.gap >= 0.0);
                if !g.foe.def().boss {
                    let first = def.foes.iter().find(|(f, _)| *f == g.foe).map(|(_, n)| *n as usize);
                    assert!(first.is_some_and(|n| n <= w + 1), "stage {i} wave {w}: {:?} is early or uninvited", g.foe);
                }
            }
        }
        // everyone invited turns up, and the boss comes last
        for (f, _) in def.foes {
            assert!(ws.iter().any(|w| w.groups.iter().any(|g| g.foe == *f)), "stage {i}: {f:?} never appears");
        }
        let boss_wave = ws.iter().position(|w| w.groups.iter().any(|g| g.foe.def().boss));
        assert_eq!(boss_wave, def.boss.map(|_| ws.len() - 1));
        assert_eq!(def.boss.is_some(), is_finale(i), "stage {i}: bosses belong to the last stage of a biome");
    }
}

#[test]
fn tables_are_in_enum_order() {
    for i in 0..FOE_COUNT {
        assert_eq!(Foe::from_index(i) as usize, i);
    }
    assert_eq!(Tw::Eyrie as usize, TOWERS.len() - 1);
    for b in &BIOMES {
        assert!(b.towers.len() >= 4);
        // every biome can answer fliers and can slow the ground
        assert!(b.towers.iter().any(|t| t.def().air && t.def().dmg > 0.0));
    }
}

// ---- points -----------------------------------------------------------------

/// There must be enough points to finish a branch, and never enough for
/// everything: reaching the top of one means going without elsewhere.
#[test]
fn skill_points_force_a_choice() {
    let earnable = LEVEL_COUNT as u32;
    for hero in HEROES {
        let tiers = hero.tiers();
        assert_eq!(hero.def().tree.len(), 3);
        assert!(earnable >= branch_cost(tiers), "{hero:?} can never finish a branch");
        assert!(earnable < 2 * branch_cost(tiers), "{hero:?} can finish two branches");
        assert!(earnable < hero.tree_cost());
    }
    for friend in FRIENDS {
        assert_eq!(friend.def().tree.len(), 2);
        assert!(earnable >= branch_cost(friend.tiers()), "{friend:?} can never finish a branch");
    }
    // two followers share the bond points: not enough to finish both trees
    let both: u32 = FRIENDS.iter().map(|f| f.tree_cost()).min().unwrap() * FOLLOWERS as u32;
    assert!(earnable < both, "bond points cover two whole trees");
}

#[test]
fn every_skill_does_something() {
    for who in HEROES.into_iter().chain(FRIENDS) {
        let base = format!("{:?}", muster(&[(who, 0)]));
        for b in 0..who.def().tree.len() {
            for t in 0..who.tiers() {
                let (skill, bit) = who.skill(b, t);
                assert!(!skill.mods.is_empty(), "{} has no effect", skill.name);
                assert!(skill.name.is_ascii() && skill.text.is_ascii(), "{} has text the game font cannot draw", skill.name);
                let with = format!("{:?}", muster(&[(who, bit)]));
                // gifts to the rest of the pack only show with someone to give to
                let other = if who == Cr::Fox { Cr::Badger } else { Cr::Fox };
                let pair = format!("{:?}", muster(&[(who, bit), (other, 0)]));
                let pair_base = format!("{:?}", muster(&[(who, 0), (other, 0)]));
                assert!(with != base || pair != pair_base, "{} changes nothing", skill.name);
            }
        }
    }
}

/// Whatever the player does between stages, points are never lost or
/// invented: spent plus unspent is what has been earned, and a follower
/// who goes home hands every bond point back.
#[test]
fn points_always_add_up() {
    for seed in 0..300u64 {
        let mut rng = Rng::new(seed);
        let mut p = Profile::new(HEROES[rng.below(3) as usize]);
        for _ in 0..400 {
            match rng.below(6) {
                0 => {
                    let l = p.frontier();
                    if p.stars[l] == 0 || rng.chance(0.2) {
                        p.record(l, 1 + rng.below(3) as u8);
                    }
                }
                1 => {
                    let hero = p.hero;
                    let _ = p.learn(hero, rng.below(3) as usize, rng.below(4) as usize);
                }
                2 => {
                    let who = FRIENDS[rng.below(5) as usize];
                    let _ = p.learn(who, rng.below(2) as usize, rng.below(3) as usize);
                }
                3 => {
                    p.join(FRIENDS[rng.below(5) as usize], rng.below(FOLLOWERS as u32) as usize);
                }
                4 => {
                    let slot = rng.below(FOLLOWERS as u32) as usize;
                    let before = p.bond_free();
                    let had = p.party[slot].map(|w| biome_core::creature::spent(w, p.skills_of(w)));
                    p.dismiss(slot);
                    if let Some(had) = had {
                        assert_eq!(p.bond_free(), before + had, "bond points did not come back");
                    }
                }
                _ => {
                    // saving and loading changes nothing
                    assert_eq!(Profile::decode(&p.encode()).as_ref(), Some(&p), "{}", p.encode());
                }
            }
            assert_eq!(p.skill_spent() + p.skill_free(), p.skill_points());
            assert_eq!(p.bond_spent() + p.bond_free(), p.bond_points());
            assert_eq!(p.skill_points(), p.cleared() as u32);
            for (i, &f) in FRIENDS.iter().enumerate() {
                if !p.in_party(f) {
                    assert_eq!(p.friend_skills[i], 0, "{f:?} kept skills after going home");
                }
                if p.in_party(f) {
                    assert!(p.is_friend(f), "{f:?} is walking with the pack without having been met");
                }
            }
            assert!(p.party[0].is_none() || p.party[0] != p.party[1]);
        }
    }
}

#[test]
fn the_third_friend_arrives_with_game_left() {
    // the pack holds two followers, so the third friend forces a choice:
    // that has to happen while there are still stages to play
    let mut p = Profile::new(Cr::Fox);
    let mut met = 0;
    for l in 0..LEVEL_COUNT {
        p.record(l, 3);
        let now = FRIENDS.iter().filter(|&&f| p.is_friend(f)).count();
        if now > met {
            met = now;
            if met == FOLLOWERS + 1 {
                assert!(LEVEL_COUNT - (l + 1) >= 2 * PER_BIOME, "the third friend arrives too late to matter");
            }
        }
    }
    assert_eq!(met, FRIENDS.len());
}

#[test]
fn a_bad_save_is_refused_or_repaired() {
    assert!(Profile::decode("").is_none());
    assert!(Profile::decode("v2|1|1|000").is_none());
    assert!(Profile::decode("b1|7|1|000000000000000000|0|--|0|0,0,0,0,0|0").is_none(), "a resident cannot be the player");
    // more skills than points: wiped rather than trusted
    let p = Profile::decode("b1|0|1|100000000000000000|fff|--|0|0,0,0,0,0|0").unwrap();
    assert_eq!(p.hero_skills, 0);
    // a follower who was never met does not get to stay
    let p = Profile::decode("b1|0|1|111000000000000000|0|03|1|7,0,0,7,0|0").unwrap();
    assert_eq!(p.party, [Some(Cr::Rabbit), None]);
    assert_eq!(p.friend_skills[3], 0);
}

// ---- whole stages -----------------------------------------------------------

#[test]
fn doing_nothing_loses_every_stage() {
    for level in 0..LEVEL_COUNT {
        for hero in HEROES {
            let sim = play(Sim::new(&setup(level, hero, Difficulty::Easy, 3)), Style::Idle);
            assert_eq!(sim.phase, Phase::Lost, "stage {level} can be won on Easy as {hero:?} without building anything");
        }
    }
}

#[test]
fn the_stand_in_wins_every_stage_on_easy_and_normal() {
    for level in 0..LEVEL_COUNT {
        for hero in HEROES {
            for d in [Difficulty::Easy, Difficulty::Normal] {
                let sim = play(Sim::new(&setup(level, hero, d, 7)), Style::Careful);
                assert_eq!(sim.phase, Phase::Won, "stage {level} lost on {d:?} as {hero:?}");
            }
        }
    }
}

#[test]
fn a_stage_plays_the_same_way_twice() {
    for level in [0, 5, 11, 17] {
        let a = play(Sim::new(&setup(level, Cr::Badger, Difficulty::Hard, 42)), Style::Careful);
        let b = play(Sim::new(&setup(level, Cr::Badger, Difficulty::Hard, 42)), Style::Careful);
        assert_eq!((a.phase, a.hearts, a.seeds, a.shooed, a.time.to_bits()), (b.phase, b.hearts, b.seeds, b.shooed, b.time.to_bits()));
    }
}

/// A player who taps anything at any time must never break the rules or
/// the program: every stage, random actions, checked every tick.
#[test]
fn random_play_never_breaks_anything() {
    for level in 0..LEVEL_COUNT {
        for seed in 0..6u64 {
            let mut rng = Rng::new(seed * 131 + level as u64);
            let hero = HEROES[rng.below(3) as usize];
            let mut p = expected_profile(level, hero);
            // every skill there is, to reach every rule
            p.hero_skills = 0xfff;
            for (i, f) in FRIENDS.iter().enumerate() {
                if p.in_party(*f) {
                    p.friend_skills[i] = 0x3f;
                }
            }
            let mut sim = Sim::new(&Setup { level, difficulty: Difficulty::from_index(rng.below(3) as usize), pack: p.pack(), guests: p.guests(level), seed });
            let towers = BIOMES[sim.def.biome].towers;
            let mounds = sim.towers.len();
            let mut spent = 0i64;
            let start = sim.seeds as i64;
            for tick in 0..60 * 60 * 6 {
                if rng.chance(0.04) {
                    let before = sim.seeds as i64;
                    match rng.below(8) {
                        0 | 1 | 2 => {
                            sim.build(rng.below(mounds as u32) as usize, rng.pick(towers));
                        }
                        3 | 4 => {
                            sim.upgrade(rng.below(mounds as u32) as usize);
                        }
                        5 => {
                            if rng.chance(0.15) {
                                sim.sell(rng.below(mounds as u32) as usize);
                            }
                        }
                        6 => sim.order(rng.below(sim.units.len() as u32) as usize, v2(rng.range(-2.0, 11.0), rng.range(-2.0, 15.0))),
                        7 => {
                            sim.cast(rng.below(sim.units.len() as u32 + 1) as usize);
                        }
                        _ => {}
                    }
                    spent += before - sim.seeds as i64;
                    if rng.chance(0.1) {
                        sim.call();
                    }
                }
                sim.tick();
                // nothing invented, nothing lost
                assert_eq!(sim.seeds as i64, start + sim.earned as i64 - spent, "stage {level} seed {seed} tick {tick}: seeds do not add up");
                assert!(sim.hearts <= sim.max_hearts);
                for e in &sim.foes {
                    assert!(e.hp.is_finite() && e.pos.x.is_finite() && e.pos.y.is_finite() && e.dist >= 0.0, "stage {level}: a broken invader");
                    assert!(e.hp <= e.max_hp + 0.01);
                }
                for u in &sim.units {
                    assert!(u.hp.is_finite() && u.hp <= u.st.hp + 0.01 && u.pos.x.is_finite());
                    assert!((0.0..=9.0).contains(&u.pos.x) && (0.0..=13.0).contains(&u.pos.y), "stage {level}: a creature left the map");
                }
                for t in sim.towers.iter().flatten() {
                    assert!((1..=3).contains(&t.level));
                }
                assert!(sim.foes.len() < 2000 && sim.shots.len() < 2000 && sim.events.len() < 5000);
                sim.events.clear();
                if !sim.playing() {
                    break;
                }
            }
            assert!(!sim.playing() || sim.time > 300.0, "stage {level} neither ended nor ran on");
        }
    }
}

// ---- particular rules ---------------------------------------------------------

fn bare(level: usize) -> Sim {
    Sim::new(&Setup { level, difficulty: Difficulty::Normal, pack: vec![(Cr::Owl, 0)], guests: vec![], seed: 1 })
}

#[test]
fn building_costs_seeds_and_selling_gives_most_back() {
    let mut sim = bare(0);
    let had = sim.seeds;
    assert!(sim.build(0, Tw::Thistle));
    assert_eq!(sim.seeds, had - 50);
    assert!(!sim.build(0, Tw::Puffball), "one structure per mound");
    assert_eq!(sim.sell_value(0), 35);
    assert!(sim.sell(0));
    assert_eq!(sim.seeds, had - 15);
    assert!(sim.towers[0].is_none());
    // Mudge's last Craft skill gives everything back
    let mut sim = Sim::new(&Setup { level: 0, difficulty: Difficulty::Normal, pack: vec![(Cr::Fox, 0), (Cr::Beaver, 0b111_000)], guests: vec![], seed: 1 });
    let had = sim.seeds;
    assert!(sim.build(0, Tw::Thistle));
    assert_eq!(sim.price(Tw::Thistle, 1), 45, "Good Wood takes a tenth off");
    assert!(sim.sell(0));
    assert_eq!(sim.seeds, had);
}

#[test]
fn nothing_can_be_bought_without_the_seeds() {
    let mut sim = bare(0);
    let mut built = 0;
    for spot in 0..sim.towers.len() {
        if sim.build(spot, Tw::Beehive) {
            built += 1;
        }
    }
    assert_eq!(built, 1, "130 seeds buys one 80-seed hive");
    assert!(!sim.upgrade(0));
}

#[test]
fn fliers_pass_over_fighters_and_rollers_roll_past() {
    // a badger square on the trail holds walkers, not wasps
    let mut sim = Sim::new(&Setup { level: 1, difficulty: Difficulty::Easy, pack: vec![(Cr::Badger, 0)], guests: vec![], seed: 1 });
    let p = sim.map.routes[0].at(6.0).0;
    sim.order(0, p);
    sim.call();
    let mut held_walker = false;
    for _ in 0..60 * 200 {
        sim.tick();
        for e in &sim.foes {
            if e.held.is_some() {
                assert!(!e.foe.def().flying && !e.foe.rolls(), "{:?} was held", e.foe);
                held_walker = true;
            }
        }
        if !sim.playing() {
            break;
        }
    }
    assert!(held_walker);
}

#[test]
fn stars_follow_the_hearts_left() {
    let mut sim = bare(0);
    assert_eq!(sim.stars(), 3);
    sim.hearts = 9;
    assert_eq!(sim.stars(), 3);
    sim.hearts = 8;
    assert_eq!(sim.stars(), 2);
    sim.hearts = 5;
    assert_eq!(sim.stars(), 2);
    sim.hearts = 4;
    assert_eq!(sim.stars(), 1);
}

#[test]
fn residents_fight_for_their_own_home_and_everyone_comes_at_the_end() {
    let p = expected_profile(1, Cr::Fox);
    assert_eq!(p.guests(1), vec![Cr::Rabbit]);
    // once she is walking with the pack she is not also a guest
    let p = expected_profile(4, Cr::Fox);
    assert!(p.in_party(Cr::Rabbit));
    assert!(p.guests(0).is_empty());
    let p = expected_profile(17, Cr::Fox);
    let guests = p.guests(17);
    assert_eq!(guests.len() + p.party.iter().flatten().count(), 5, "all five friends are at the last stand");
    assert!(p.guests(16).is_empty());
}
