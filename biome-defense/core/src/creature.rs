//! The creatures: three wanderers to play as, and five residents who can
//! join the pack. Each has an attack, one ability, and a skill tree.
//!
//! A tree is a few branches of skills learned in order, each costing one
//! point more than the last. There are never enough points for the whole
//! tree, so reaching the end of one branch means leaving another short.

use crate::defs::Look;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Cr {
    Fox,
    Badger,
    Owl,
    Rabbit,
    Squirrel,
    Beaver,
    Tortoise,
    Marmot,
}

pub const CREATURE_COUNT: usize = 8;
/// The three a new game offers.
pub const HEROES: [Cr; 3] = [Cr::Fox, Cr::Badger, Cr::Owl];
/// The residents, in the order their biomes are crossed.
pub const FRIENDS: [Cr; 5] = [Cr::Rabbit, Cr::Squirrel, Cr::Beaver, Cr::Tortoise, Cr::Marmot];

/// Structures within this many tiles of a creature get its aura skills.
pub const AURA_R: f32 = 2.3;
/// Seconds a knocked-out creature takes to get back up, before skills.
pub const REVIVE: f32 = 10.0;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Ability {
    /// Leap onto the toughest invader in range.
    Pounce { range: f32, dmg: f32, stun: f32 },
    /// Hit and stun everything on the ground around you.
    Slam { radius: f32, dmg: f32, stun: f32 },
    /// Blow everything nearby back along the trail.
    Gust { radius: f32, push: f32, dmg: f32 },
    /// Stun everything on the ground around you.
    Thump { radius: f32, stun: f32 },
    /// Rain `count` shots on invaders in range.
    Storm { range: f32, count: u8, dmg: f32 },
    /// Block the trail with a dam for a while.
    Dam { hp: f32, secs: f32 },
    /// Pull in: nothing can hurt you, and you hold more invaders.
    Shell { secs: f32 },
    /// Structures nearby work faster for a while.
    Whistle { radius: f32, haste: f32, secs: f32 },
}

/// Skills too particular to be a number. `sim` knows what each one does.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Flag {
    Frenzy,
    Foxfire,
    Decoy,
    Bounce,
    Rampage,
    Aftershock,
    Sinkhole,
    LastStand,
    Dive,
    ColdFront,
    Twister,
    Quake,
    Cousins,
    BigStorm,
    Conkers,
    Splinters,
    Flood,
    Thorns,
    ShellHeal,
    Mend,
    Shrill,
    Rally,
}

/// One effect of a skill.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum M {
    /// Attack damage, as a fraction more.
    Dmg(f32),
    /// Attacks per second, as a fraction more.
    Rate(f32),
    Range(f32),
    Hp(f32),
    Speed(f32),
    Armor(f32),
    /// Holds this many more invaders.
    Block(u8),
    /// Heals this much faster (1.0 = twice as fast).
    Regen(f32),
    /// Recovers from a knockout this fraction sooner.
    Revive(f32),
    /// Ability recharges this fraction sooner.
    Cd(f32),
    /// Ability hits (or pushes, or holds) this fraction harder.
    Power(f32),
    /// Ability reaches this fraction farther.
    Area(f32),
    /// Ability lasts this many seconds longer.
    Last(f32),
    /// Attacks hit everything within this radius of the target.
    Splash(f32),
    /// Attacks slow by a fraction for some seconds.
    Slow(f32, f32),
    /// Attacks add damage per second for some seconds.
    Poison(f32, f32),
    /// Chance of a critical hit, and its multiplier.
    Crit(f32, f32),
    /// Attacks strip armor for some seconds.
    Shred(f32, f32),
    /// Attacks hit this many more invaders.
    Extra(u8),
    /// Attacks heal this fraction of the damage done.
    Leech(f32),
    /// What it hits takes this fraction more damage from everything.
    Mark(f32, f32),
    /// Attacks knock back this many tiles.
    Shove(f32),
    /// Invaders yield this fraction more seeds.
    Bounty(f32),
    /// Seeds at the start of a stage.
    Seeds(u32),
    /// Seeds at the start of every wave.
    WaveSeeds(u32),
    /// Structures cost this fraction less.
    Cheap(f32),
    /// The home has this many more hearts.
    Hearts(u32),
    /// Selling a structure gives everything back.
    Refund,
    /// Every structure hits this fraction harder.
    AllDmg(f32),
    /// Structures near this creature: damage, rate, range.
    AuraDmg(f32),
    AuraRate(f32),
    AuraRange(f32),
    /// The rest of the pack: damage, health, knockout recovery, healing.
    PackDmg(f32),
    PackHp(f32),
    PackRevive(f32),
    PackRegen(f32),
    Flag(Flag),
}

/// The picture on a skill's button.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Icon {
    Fang,
    Paw,
    Heart,
    Shield,
    Wind,
    Seed,
    Star,
    Hammer,
    Eye,
    Bolt,
    Flame,
    Pack,
}

pub struct Skill {
    pub name: &'static str,
    pub text: &'static str,
    pub icon: Icon,
    pub mods: &'static [M],
}

pub struct Branch {
    pub name: &'static str,
    /// A word on what the branch is for.
    pub about: &'static str,
    pub skills: &'static [Skill],
}

pub struct CreatureDef {
    pub name: &'static str,
    pub species: &'static str,
    pub role: &'static str,
    pub blurb: &'static str,
    pub hp: f32,
    /// Tiles per second.
    pub speed: f32,
    pub dmg: f32,
    pub rate: f32,
    pub range: f32,
    /// How many invaders it can hold up at once. Zero for those who keep
    /// their distance.
    pub block: u8,
    pub armor: f32,
    /// Fights from a perch or the air: nothing on the ground can reach it.
    pub perched: bool,
    /// Can hit fliers.
    pub air: bool,
    pub look: Look,
    pub ability: Ability,
    pub ability_name: &'static str,
    pub ability_text: &'static str,
    pub cooldown: f32,
    pub tree: &'static [Branch],
}

const fn sk(name: &'static str, text: &'static str, icon: Icon, mods: &'static [M]) -> Skill {
    Skill { name, text, icon, mods }
}

use Icon::*;

static FOX: CreatureDef = CreatureDef {
    name: "Rust",
    species: "fox",
    role: "Skirmisher",
    blurb: "Quick and sharp. Goes for the biggest threat in sight, then falls back.",
    hp: 230.0,
    speed: 3.4,
    dmg: 21.0,
    rate: 1.5,
    range: 0.75,
    block: 1,
    armor: 2.0,
    perched: false,
    air: false,
    look: Look::None,
    ability: Ability::Pounce { range: 3.0, dmg: 85.0, stun: 1.2 },
    ability_name: "Pounce",
    ability_text: "Leaps onto the toughest invader in reach, hitting hard and stunning it.",
    cooldown: 9.5,
    tree: &[
        Branch {
            name: "Fang",
            about: "the bite",
            skills: &[
                sk("Sharp Teeth", "Bites hit 35% harder.", Fang, &[M::Dmg(0.35)]),
                sk("Quick Paws", "Attacks 30% faster and runs 20% faster.", Paw, &[M::Rate(0.3), M::Speed(0.2)]),
                sk("Hamstring", "Bites slow the invader by 35% and leave it bleeding.", Fang, &[M::Slow(0.35, 2.0), M::Poison(8.0, 3.0)]),
                sk("Frenzy", "Every invader Rust kills makes him bite faster and takes 2s off Pounce.", Flame, &[M::Flag(Flag::Frenzy)]),
            ],
        },
        Branch {
            name: "Trickster",
            about: "the pounce",
            skills: &[
                sk("Long Leap", "Pounce reaches 40% farther and recharges 15% sooner.", Wind, &[M::Area(0.4), M::Cd(0.15)]),
                sk("Foxfire", "Pounce lands in a burst of flame that scorches everything nearby. Plants burn worst.", Flame, &[M::Flag(Flag::Foxfire)]),
                sk("Decoy", "Pouncing leaves a decoy behind that holds up to 3 invaders for 5s.", Star, &[M::Flag(Flag::Decoy)]),
                sk("Three Tails", "Pounce bounces on to two more invaders.", Bolt, &[M::Flag(Flag::Bounce)]),
            ],
        },
        Branch {
            name: "Forager",
            about: "forage and the pack",
            skills: &[
                sk("Good Nose", "Invaders yield 15% more forage.", Seed, &[M::Bounty(0.15)]),
                sk("Hidden Cache", "Start every stage with 40 extra forage.", Seed, &[M::Seeds(40)]),
                sk("Sly Builder", "Structures cost 12% less. Those near Rust work 20% faster.", Hammer, &[M::Cheap(0.12), M::AuraRate(0.2)]),
                sk("Pack Leader", "The rest of the pack hits 30% harder with 30% more health. The home can take 3 more breaches.", Pack, &[M::PackDmg(0.3), M::PackHp(0.3), M::Hearts(3)]),
            ],
        },
    ],
};

static BADGER: CreatureDef = CreatureDef {
    name: "Brock",
    species: "badger",
    role: "Bruiser",
    blurb: "Slow, wide and stubborn. Holds three invaders at once and makes them pay for it.",
    hp: 360.0,
    speed: 2.3,
    dmg: 27.0,
    rate: 0.95,
    range: 0.75,
    block: 3,
    armor: 1.0,
    perched: false,
    air: false,
    look: Look::None,
    ability: Ability::Slam { radius: 1.4, dmg: 55.0, stun: 1.5 },
    ability_name: "Ground Slam",
    ability_text: "Slams the ground: everything walking nearby is hurt and stunned.",
    cooldown: 14.0,
    tree: &[
        Branch {
            name: "Claw",
            about: "the swipe",
            skills: &[
                sk("Heavy Paws", "Swipes hit 35% harder.", Fang, &[M::Dmg(0.35)]),
                sk("Cleave", "Each swipe hits everything close to the target.", Paw, &[M::Splash(0.85)]),
                sk("Can Opener", "Swipes strip 5 armor for 4s. Good against shells and machines.", Hammer, &[M::Shred(5.0, 4.0)]),
                sk("Rampage", "While holding two or more invaders, Brock swipes 70% faster and his hits heal him.", Flame, &[M::Flag(Flag::Rampage)]),
            ],
        },
        Branch {
            name: "Earth",
            about: "the slam",
            skills: &[
                sk("Wide Slam", "Ground Slam reaches 30% farther.", Wind, &[M::Area(0.3)]),
                sk("Aftershock", "Slammed ground stays cracked for 4s, slowing whatever crosses it by 45%.", Bolt, &[M::Flag(Flag::Aftershock)]),
                sk("Deep Breath", "Ground Slam hits 60% harder and recharges 25% sooner.", Star, &[M::Power(0.6), M::Cd(0.25)]),
                sk("Sinkhole", "Ground Slam drags invaders a tile and a half back the way they came.", Flame, &[M::Flag(Flag::Sinkhole)]),
            ],
        },
        Branch {
            name: "Bulwark",
            about: "staying up",
            skills: &[
                sk("Thick Hide", "35% more health and 3 more armor.", Shield, &[M::Hp(0.35), M::Armor(3.0)]),
                sk("Dig In", "Holds 2 more invaders and heals twice as fast.", Heart, &[M::Block(2), M::Regen(1.0)]),
                sk("Back on Your Feet", "Recovers from a knockout twice as fast, and so does the rest of the pack.", Pack, &[M::Revive(0.5), M::PackRevive(0.5)]),
                sk("Unbreakable", "Instead of going down, Brock shrugs it off: 6s unstoppable, then full health. Once every 45s.", Star, &[M::Flag(Flag::LastStand)]),
            ],
        },
    ],
};

static OWL: CreatureDef = CreatureDef {
    name: "Nyx",
    species: "owl",
    role: "Sharpshooter",
    blurb: "Strikes from above, where nothing can reach her. Holds nobody up, but hits anything, even fliers.",
    hp: 100.0,
    speed: 4.2,
    dmg: 21.0,
    rate: 1.0,
    range: 2.6,
    block: 0,
    armor: 0.0,
    perched: true,
    air: true,
    look: Look::Feather,
    ability: Ability::Gust { radius: 1.7, push: 1.6, dmg: 26.0 },
    ability_name: "Gust",
    ability_text: "A beat of her wings blows everything nearby back along the trail.",
    cooldown: 12.0,
    tree: &[
        Branch {
            name: "Talon",
            about: "the strike",
            skills: &[
                sk("Sharp Talons", "Strikes hit 35% harder.", Fang, &[M::Dmg(0.35)]),
                sk("Night Eyes", "Sees 25% farther. One strike in four is a critical for triple damage.", Eye, &[M::Range(0.25), M::Crit(0.25, 3.0)]),
                sk("Twin Feathers", "Strikes two invaders at once.", Bolt, &[M::Extra(1)]),
                sk("Sky Dive", "Every fourth strike is a dive: four times the damage to everything where she lands.", Flame, &[M::Flag(Flag::Dive)]),
            ],
        },
        Branch {
            name: "Gale",
            about: "the gust",
            skills: &[
                sk("Strong Wings", "Gust reaches 25% farther and shoves 40% harder.", Wind, &[M::Area(0.25), M::Power(0.4)]),
                sk("Cold Front", "Gusted invaders stay slowed by 45% for 4s.", Bolt, &[M::Flag(Flag::ColdFront)]),
                sk("Second Wind", "Gust recharges 30% sooner.", Star, &[M::Cd(0.3)]),
                sk("Twister", "Gust leaves a whirlwind that keeps shoving invaders back for 4s.", Flame, &[M::Flag(Flag::Twister)]),
            ],
        },
        Branch {
            name: "Wisdom",
            about: "the structures",
            skills: &[
                sk("Night Watch", "Structures near Nyx reach 15% farther.", Eye, &[M::AuraRange(0.15)]),
                sk("Mark Prey", "Whatever she strikes takes 25% more damage from everything for 4s.", Star, &[M::Mark(0.25, 4.0)]),
                sk("Night Call", "Structures near Nyx work 25% faster.", Hammer, &[M::AuraRate(0.25)]),
                sk("Omen", "Every wave begins with 20 extra forage, and every structure hits 20% harder.", Seed, &[M::WaveSeeds(20), M::AllDmg(0.2)]),
            ],
        },
    ],
};

static RABBIT: CreatureDef = CreatureDef {
    name: "Sorrel",
    species: "hare",
    role: "Scrapper",
    blurb: "Small and fast. Kicks twice a second and does not back down.",
    hp: 120.0,
    speed: 3.8,
    dmg: 9.0,
    rate: 2.0,
    range: 0.7,
    block: 1,
    armor: 0.0,
    perched: false,
    air: false,
    look: Look::None,
    ability: Ability::Thump { radius: 1.3, stun: 1.6 },
    ability_name: "Thump",
    ability_text: "Thumps the ground: everything walking nearby is stunned.",
    cooldown: 10.0,
    tree: &[
        Branch {
            name: "Kick",
            about: "the fight",
            skills: &[
                sk("Strong Legs", "Kicks hit 40% harder.", Paw, &[M::Dmg(0.4)]),
                sk("Boot", "Kicks shove invaders back along the trail.", Wind, &[M::Shove(0.25)]),
                sk("Earthshaker", "Thump also hurts everything it stuns, and recharges 30% sooner.", Flame, &[M::Flag(Flag::Quake), M::Cd(0.3)]),
            ],
        },
        Branch {
            name: "Warren",
            about: "the warren",
            skills: &[
                sk("Gleaner", "Invaders yield 10% more forage.", Seed, &[M::Bounty(0.1)]),
                sk("Tunnels", "Moves twice as fast and recovers from knockouts 60% sooner.", Bolt, &[M::Speed(1.0), M::Revive(0.6)]),
                sk("Kin", "Thump calls two of the warren, who each hold an invader for 6s.", Pack, &[M::Flag(Flag::Cousins)]),
            ],
        },
    ],
};

static SQUIRREL: CreatureDef = CreatureDef {
    name: "Hazel",
    species: "squirrel",
    role: "Slinger",
    blurb: "Throws from the branches, where nothing can reach her. Hits fliers.",
    hp: 100.0,
    speed: 3.6,
    dmg: 12.0,
    rate: 1.25,
    range: 2.4,
    block: 0,
    armor: 0.0,
    perched: true,
    air: true,
    look: Look::Acorn,
    ability: Ability::Storm { range: 3.0, count: 8, dmg: 20.0 },
    ability_name: "Acorn Storm",
    ability_text: "Empties her cache: eight acorns come down on invaders in reach.",
    cooldown: 11.0,
    tree: &[
        Branch {
            name: "Aim",
            about: "the throw",
            skills: &[
                sk("Good Arm", "Acorns hit 35% harder.", Fang, &[M::Dmg(0.35)]),
                sk("Treetop View", "Throws 25% farther. One acorn in four cracks for triple damage.", Eye, &[M::Range(0.25), M::Crit(0.25, 3.0)]),
                sk("Both Paws", "Throws at two invaders at once.", Bolt, &[M::Extra(1)]),
            ],
        },
        Branch {
            name: "Hoard",
            about: "the storm",
            skills: &[
                sk("Stash", "Start every stage with 30 extra forage.", Seed, &[M::Seeds(30)]),
                sk("Bigger Storm", "Acorn Storm drops 5 more acorns and recharges 20% sooner.", Star, &[M::Flag(Flag::BigStorm), M::Cd(0.2)]),
                sk("Shatter", "Storm acorns burst, hitting everything nearby and stunning for half a second.", Flame, &[M::Flag(Flag::Conkers)]),
            ],
        },
    ],
};

static BEAVER: CreatureDef = CreatureDef {
    name: "Alder",
    species: "beaver",
    role: "Builder",
    blurb: "Blocks the trail with a dam when it matters, and makes structures better by standing near them.",
    hp: 260.0,
    speed: 2.1,
    dmg: 17.0,
    rate: 0.9,
    range: 0.75,
    block: 2,
    armor: 0.0,
    perched: false,
    air: false,
    look: Look::None,
    ability: Ability::Dam { hp: 320.0, secs: 8.0 },
    ability_name: "Dam",
    ability_text: "Drops a log dam across the nearest trail. Everything on foot stops to chew through it.",
    cooldown: 16.0,
    tree: &[
        Branch {
            name: "Timber",
            about: "the dam",
            skills: &[
                sk("Stout Logs", "The dam is 60% tougher and stands 3s longer.", Shield, &[M::Power(0.6), M::Last(3.0)]),
                sk("Splinters", "Invaders chewing on the dam take 10 damage a second.", Fang, &[M::Flag(Flag::Splinters)]),
                sk("Flood", "When the dam goes, the water behind it washes invaders two tiles back.", Wind, &[M::Flag(Flag::Flood)]),
            ],
        },
        Branch {
            name: "Craft",
            about: "the structures",
            skills: &[
                sk("Good Wood", "Structures cost 10% less.", Seed, &[M::Cheap(0.1)]),
                sk("Fine Joinery", "Structures near Alder hit 25% harder.", Hammer, &[M::AuraDmg(0.25)]),
                sk("Master Builder", "Structures near Alder also work 25% faster. Selling returns the full cost.", Star, &[M::AuraRate(0.25), M::Refund]),
            ],
        },
    ],
};

static TORTOISE: CreatureDef = CreatureDef {
    name: "Mesa",
    species: "tortoise",
    role: "Wall",
    blurb: "A hundred summers old. Holds four invaders at once and is in no hurry about any of it.",
    hp: 540.0,
    speed: 1.3,
    dmg: 24.0,
    rate: 0.6,
    range: 0.75,
    block: 4,
    armor: 3.0,
    perched: false,
    air: false,
    look: Look::None,
    ability: Ability::Shell { secs: 5.0 },
    ability_name: "Shell Up",
    ability_text: "Pulls in. Nothing can hurt her, and she holds three more invaders.",
    cooldown: 15.0,
    tree: &[
        Branch {
            name: "Shell",
            about: "the wall",
            skills: &[
                sk("Old Shell", "30% more health and 2 more armor.", Shield, &[M::Hp(0.3), M::Armor(2.0)]),
                sk("Spikes", "Whatever bites Mesa takes 8 damage a second for it.", Fang, &[M::Flag(Flag::Thorns)]),
                sk("Fortress", "Shell Up lasts 3s longer and heals her completely.", Star, &[M::Last(3.0), M::Flag(Flag::ShellHeal)]),
            ],
        },
        Branch {
            name: "Elder",
            about: "the pack",
            skills: &[
                sk("Slow and Steady", "Hits 30% harder. Walks 25% faster, which is still not fast.", Paw, &[M::Dmg(0.3), M::Speed(0.25)]),
                sk("Old Stories", "The rest of the pack heals twice as fast and recovers from knockouts 40% sooner.", Pack, &[M::PackRegen(1.0), M::PackRevive(0.4)]),
                sk("Hundred Summers", "The home can take 3 more breaches, and mends one after every wave.", Heart, &[M::Hearts(3), M::Flag(Flag::Mend)]),
            ],
        },
    ],
};

static MARMOT: CreatureDef = CreatureDef {
    name: "Tarn",
    species: "marmot",
    role: "Lookout",
    blurb: "Keeps watch from a rock and whistles the structures into a hurry. Throws stones at fliers.",
    hp: 100.0,
    speed: 2.9,
    dmg: 9.0,
    rate: 1.2,
    range: 2.2,
    block: 0,
    armor: 0.0,
    perched: true,
    air: true,
    look: Look::Pebble,
    ability: Ability::Whistle { radius: 2.6, haste: 0.6, secs: 5.0 },
    ability_name: "Whistle",
    ability_text: "A piercing whistle: structures in earshot work 60% faster for 5s.",
    cooldown: 14.0,
    tree: &[
        Branch {
            name: "Whistle",
            about: "the alarm",
            skills: &[
                sk("Big Lungs", "Whistle reaches 30% farther and lasts 2s longer.", Wind, &[M::Area(0.3), M::Last(2.0)]),
                sk("Shrill", "Whistle stuns every flier in earshot for 2s.", Bolt, &[M::Flag(Flag::Shrill)]),
                sk("Rally", "Whistle reaches every structure on the map and takes 4s off the pack's abilities.", Pack, &[M::Flag(Flag::Rally)]),
            ],
        },
        Branch {
            name: "Lookout",
            about: "the watch",
            skills: &[
                sk("Sharp Eyes", "Structures near Tarn reach 20% farther.", Eye, &[M::AuraRange(0.2)]),
                sk("Early Warning", "Every wave begins with 12 extra forage.", Seed, &[M::WaveSeeds(12)]),
                sk("Sentinel", "Pebbles hit twice as hard, slow by 40% and strike two invaders.", Star, &[M::Dmg(1.0), M::Slow(0.4, 2.0), M::Extra(1)]),
            ],
        },
    ],
};

impl Cr {
    pub fn def(self) -> &'static CreatureDef {
        match self {
            Cr::Fox => &FOX,
            Cr::Badger => &BADGER,
            Cr::Owl => &OWL,
            Cr::Rabbit => &RABBIT,
            Cr::Squirrel => &SQUIRREL,
            Cr::Beaver => &BEAVER,
            Cr::Tortoise => &TORTOISE,
            Cr::Marmot => &MARMOT,
        }
    }

    pub fn from_index(i: usize) -> Option<Cr> {
        [Cr::Fox, Cr::Badger, Cr::Owl, Cr::Rabbit, Cr::Squirrel, Cr::Beaver, Cr::Tortoise, Cr::Marmot].get(i).copied()
    }

    pub fn is_hero(self) -> bool {
        HEROES.contains(&self)
    }

    /// Position among the residents (0 = the meadow's), if it is one.
    pub fn friend_index(self) -> Option<usize> {
        FRIENDS.iter().position(|&f| f == self)
    }

    /// Skills per branch.
    pub fn tiers(self) -> usize {
        self.def().tree[0].skills.len()
    }

    pub fn skill_count(self) -> usize {
        self.def().tree.len() * self.tiers()
    }

    /// The skill in a branch at a tier, and its bit in a skill mask.
    pub fn skill(self, branch: usize, tier: usize) -> (&'static Skill, u16) {
        (&self.def().tree[branch].skills[tier], 1 << (branch * self.tiers() + tier))
    }

    /// Points to learn every skill.
    pub fn tree_cost(self) -> u32 {
        self.def().tree.len() as u32 * branch_cost(self.tiers())
    }
}

/// Points a skill costs: one more for each step along its branch.
pub fn skill_cost(tier: usize) -> u32 {
    tier as u32 + 1
}

/// Points to learn a whole branch of `tiers` skills.
pub fn branch_cost(tiers: usize) -> u32 {
    (0..tiers).map(skill_cost).sum()
}

/// Points spent in a skill mask.
pub fn spent(who: Cr, mask: u16) -> u32 {
    let tiers = who.tiers();
    (0..who.skill_count()).filter(|i| mask & (1 << i) != 0).map(|i| skill_cost(i % tiers)).sum()
}

// ---- what the skills add up to ---------------------------------------------

/// A creature as it walks onto a stage: its base numbers with its learned
/// skills (and the rest of the pack's gifts) applied.
#[derive(Clone, Debug)]
pub struct Stats {
    pub hp: f32,
    pub speed: f32,
    pub dmg: f32,
    pub rate: f32,
    pub range: f32,
    pub block: u8,
    pub armor: f32,
    pub regen: f32,
    pub revive: f32,
    pub cd: f32,
    pub power: f32,
    pub area: f32,
    pub last: f32,
    pub splash: f32,
    pub slow: (f32, f32),
    pub poison: (f32, f32),
    pub crit: (f32, f32),
    pub shred: (f32, f32),
    pub extra: u8,
    pub leech: f32,
    pub mark: (f32, f32),
    pub shove: f32,
    pub aura_dmg: f32,
    pub aura_rate: f32,
    pub aura_range: f32,
    flags: u32,
    give: [f32; 4],
}

impl Stats {
    pub fn has(&self, f: Flag) -> bool {
        self.flags & (1 << f as u32) != 0
    }

    /// A creature with no skills learned.
    pub fn base(who: Cr) -> Stats {
        let d = who.def();
        Stats {
            hp: d.hp,
            speed: d.speed,
            dmg: d.dmg,
            rate: d.rate,
            range: d.range,
            block: d.block,
            armor: d.armor,
            regen: 1.0,
            revive: REVIVE,
            cd: d.cooldown,
            power: 1.0,
            area: 1.0,
            last: 0.0,
            splash: 0.0,
            slow: (0.0, 0.0),
            poison: (0.0, 0.0),
            crit: (0.0, 1.0),
            shred: (0.0, 0.0),
            extra: 0,
            leech: 0.0,
            mark: (0.0, 0.0),
            shove: 0.0,
            aura_dmg: 0.0,
            aura_rate: 0.0,
            aura_range: 0.0,
            flags: 0,
            give: [0.0; 4],
        }
    }
}

/// What the pack's skills do for the stage as a whole.
#[derive(Clone, Debug, Default)]
pub struct Perks {
    pub bounty: f32,
    pub seeds: u32,
    pub wave_seeds: u32,
    pub cheap: f32,
    pub hearts: u32,
    pub refund: bool,
    pub mend: bool,
    pub all_dmg: f32,
}

/// Works out everyone's numbers from who is in the pack and what each has
/// learned (`mask` has one bit per skill, see [`Cr::skill`]).
pub fn muster(pack: &[(Cr, u16)]) -> (Vec<Stats>, Perks) {
    let mut perks = Perks::default();
    let mut all: Vec<Stats> = Vec::with_capacity(pack.len());
    for &(who, mask) in pack {
        let mut s = Stats::base(who);
        let d = who.def();
        let tiers = who.tiers();
        for (b, branch) in d.tree.iter().enumerate() {
            for (t, skill) in branch.skills.iter().enumerate() {
                if mask & (1 << (b * tiers + t)) == 0 {
                    continue;
                }
                for m in skill.mods {
                    match *m {
                        M::Dmg(x) => s.dmg += d.dmg * x,
                        M::Rate(x) => s.rate += d.rate * x,
                        M::Range(x) => s.range += d.range * x,
                        M::Hp(x) => s.hp += d.hp * x,
                        M::Speed(x) => s.speed += d.speed * x,
                        M::Armor(x) => s.armor += x,
                        M::Block(n) => s.block += n,
                        M::Regen(x) => s.regen += x,
                        M::Revive(x) => s.revive *= 1.0 - x,
                        M::Cd(x) => s.cd *= 1.0 - x,
                        M::Power(x) => s.power += x,
                        M::Area(x) => s.area += x,
                        M::Last(x) => s.last += x,
                        M::Splash(x) => s.splash = s.splash.max(x),
                        M::Slow(a, b) => s.slow = (a, b),
                        M::Poison(a, b) => s.poison = (a, b),
                        M::Crit(a, b) => s.crit = (a, b),
                        M::Shred(a, b) => s.shred = (a, b),
                        M::Extra(n) => s.extra += n,
                        M::Leech(x) => s.leech += x,
                        M::Mark(a, b) => s.mark = (a, b),
                        M::Shove(x) => s.shove += x,
                        M::Bounty(x) => perks.bounty += x,
                        M::Seeds(n) => perks.seeds += n,
                        M::WaveSeeds(n) => perks.wave_seeds += n,
                        M::Cheap(x) => perks.cheap += x,
                        M::Hearts(n) => perks.hearts += n,
                        M::Refund => perks.refund = true,
                        M::AllDmg(x) => perks.all_dmg += x,
                        M::AuraDmg(x) => s.aura_dmg += x,
                        M::AuraRate(x) => s.aura_rate += x,
                        M::AuraRange(x) => s.aura_range += x,
                        M::PackDmg(x) => s.give[0] += x,
                        M::PackHp(x) => s.give[1] += x,
                        M::PackRevive(x) => s.give[2] += x,
                        M::PackRegen(x) => s.give[3] += x,
                        M::Flag(f) => {
                            s.flags |= 1 << f as u32;
                            if f == Flag::Mend {
                                perks.mend = true;
                            }
                        }
                    }
                }
            }
        }
        all.push(s);
    }
    // gifts go to everyone but the giver
    let gifts: Vec<[f32; 4]> = all.iter().map(|s| s.give).collect();
    for (i, s) in all.iter_mut().enumerate() {
        let d = pack[i].0.def();
        for (j, g) in gifts.iter().enumerate() {
            if i == j {
                continue;
            }
            s.dmg += d.dmg * g[0];
            s.hp += d.hp * g[1];
            s.revive *= 1.0 - g[2];
            s.regen += g[3];
        }
    }
    (all, perks)
}
