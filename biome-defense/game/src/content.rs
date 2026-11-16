//! The story, and the other words.
//!
//! The fonts are baked into the build as plain ASCII, so everything here
//! sticks to it: straight quotes, three dots, no dashes wider than a hyphen.

use biome_core::creature::Cr;
use biome_core::level::PER_BIOME;
use biome_core::{Profile, BIOMES};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Who {
    Narrator,
    /// Whichever creature the player chose.
    Hero,
    Cr(Cr),
}

pub struct Line {
    pub who: Who,
    pub text: &'static str,
}

/// A stretch of story told in one sitting.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Beat {
    /// Before anything: how the den was lost.
    Intro,
    /// Arriving in a biome.
    Arrive(usize),
    /// A resident's home has been held: what was learned there.
    Held(usize),
    Ending,
}

impl Beat {
    /// This beat's bit in `Profile::seen`.
    pub fn bit(self) -> u32 {
        match self {
            Beat::Intro => 1,
            Beat::Arrive(b) => 1 << (1 + b),
            Beat::Held(b) => 1 << (7 + b),
            Beat::Ending => 1 << 13,
        }
    }

    /// The biome whose colours go behind this beat.
    pub fn biome(self) -> usize {
        match self {
            Beat::Intro | Beat::Ending => 5,
            Beat::Arrive(b) | Beat::Held(b) => b,
        }
    }

    /// The resident who offers to join once this beat has been told.
    pub fn joiner(self) -> Option<Cr> {
        match self {
            Beat::Held(b) => BIOMES[b].resident,
            _ => None,
        }
    }
}

/// The next stretch of story the player has earned and not yet been told.
pub fn next_beat(p: &Profile) -> Option<Beat> {
    let seen = |b: Beat| p.seen & b.bit() != 0;
    if !seen(Beat::Intro) {
        return Some(Beat::Intro);
    }
    for b in 0..BIOMES.len() - 1 {
        if p.stars[b * PER_BIOME + PER_BIOME - 1] > 0 && !seen(Beat::Held(b)) {
            return Some(Beat::Held(b));
        }
    }
    if p.finished() {
        return (!seen(Beat::Ending)).then_some(Beat::Ending);
    }
    let front = p.frontier();
    let biome = front / PER_BIOME;
    if front % PER_BIOME == 0 && !seen(Beat::Arrive(biome)) {
        return Some(Beat::Arrive(biome));
    }
    None
}

const fn n(text: &'static str) -> Line {
    Line { who: Who::Narrator, text }
}
const fn h(text: &'static str) -> Line {
    Line { who: Who::Hero, text }
}
const fn c(who: Cr, text: &'static str) -> Line {
    Line { who: Who::Cr(who), text }
}

pub fn lines(beat: Beat, hero: Cr) -> Vec<Line> {
    use Cr::*;
    match beat {
        Beat::Intro => vec![
            n("Last spring there was a den on the hill. Yours."),
            n("The machines came at first light. By noon there was no hill."),
            h(match hero {
                Fox => "I ran. I am good at running. Not everyone in that den was.",
                Badger => "I dug in and held. It made no difference. They do not notice what they bury.",
                _ => "I watched it from the last tree standing. Then they took the tree.",
            }),
            n("Since then the ground has not stopped moving. Swarms first, driven west ahead of the noise. Whatever is behind them is coming the same way."),
            h("There is a warren in the meadow. It is next."),
            h("I could not hold mine. I can help hold theirs."),
        ],
        Beat::Arrive(0) => vec![
            c(Rabbit, "You are not from here. Good. Everyone from here is underground, and frightened."),
            c(Rabbit, "Ants on the north trail since dawn. Beetles behind them. They do not stop to feed. They just come."),
            h("They are not raiding. They are fleeing. It will look the same from your door."),
            c(Rabbit, "Then show me how to hold a door."),
        ],
        Beat::Held(0) => vec![
            c(Rabbit, "The warren stands. I counted. Everyone I started the day with."),
            h("The big one was not leading them. It was running too. Did you see how it kept looking back?"),
            c(Rabbit, "East. They all came from the east."),
            h("Then that is where I am going."),
            c(Rabbit, "Not alone. I know this ground, and I kick harder than I look. The warren can spare me. It cannot spare another spring like this one."),
        ],
        Beat::Arrive(1) => vec![
            c(Squirrel, "Far enough. This wood has had its fill of things arriving."),
            c(Rabbit, "We held the warren. We came to see what drove them at us."),
            c(Squirrel, "Then look up. That vine was not here ten days ago. It took a beech this morning. It grows where the ground has been opened, and the ground is being opened."),
        ],
        Beat::Held(1) => vec![
            c(Squirrel, "The oak holds. Four hundred winters, and it nearly went in a week."),
            c(Squirrel, "I followed the vine back to where it starts. Cut earth. A straight scar, tree to tree, with orange marks on the trunks left standing."),
            h("Marks made by what?"),
            c(Squirrel, "By something that plans. I would like to see it. Make room."),
        ],
        Beat::Arrive(2) => vec![
            c(Beaver, "Mind where you stand. That was water last month. It drops a paw's width every night."),
            c(Beaver, "There are two-legs on the far bank. They walk in a line and leave stakes behind them. Orange."),
            h("A line to where?"),
            c(Beaver, "Through my lodge. Eleven years I have built here. Nobody asked what was in the way."),
        ],
        Beat::Held(2) => vec![
            c(Beaver, "The pump is stopped. The water will come back. Slower than it left; it always is."),
            c(Beaver, "I walked their line of stakes while you slept. East to west, dead straight. It does not bend for water or for rock."),
            h("It runs through the meadow."),
            c(Beaver, "It runs through everything. Somebody drew that. I mean to find the paw that drew it."),
        ],
        Beat::Arrive(3) => vec![
            c(Tortoise, "Sit. You have walked far, and you will learn nothing from me in a hurry."),
            c(Tortoise, "A hundred summers on this rock. I have seen fire and I have seen flood. I have never seen a straight line. Nothing that lives makes one."),
            c(Tortoise, "It is coming up the canyon now. Everything in front of it runs. You have met the scorpions."),
        ],
        Beat::Held(3) => vec![
            c(Tortoise, "My thanks. I would have outlasted them. It is what I do. But I would have been alone when it was over."),
            c(Tortoise, "The line climbs from here, up the crag. From the top you can see where it starts."),
            h("Then that is where we look."),
            c(Tortoise, "I will come. Do not wait. I have never once been early, and I have never once failed to arrive."),
        ],
        Beat::Arrive(4) => vec![
            c(Marmot, "Stop there. ...No. You are not theirs. Theirs do not look up."),
            c(Marmot, "I keep watch on this slope. It used to mean eagles. Now there are machines in the air, and a track cut a little further up the mountain every day."),
            c(Marmot, "Hold the den with me and I will show you what I have been watching. You will not like it."),
        ],
        Beat::Held(4) => vec![
            c(Marmot, "There. East, past the desert. Follow the scar back."),
            n("A grey line, ruler straight, from the edge of the town: through the red rock, the marsh, the Old Wood. Pointed at the meadow."),
            h("A road."),
            h("It starts where my den was."),
            c(Marmot, "Then that is where it has to stop. I have watched long enough."),
        ],
        Beat::Arrive(_) => vec![
            n("Road's End. Stumps, churned mud, orange cones. Under all of it, somewhere, a hill."),
            h("This is where it began."),
            h("We dig in here. On their line. And we do not move."),
        ],
        Beat::Held(_) => vec![],
        Beat::Ending => vec![
            n("Big Yellow stopped with its blade a length from the den. For a long moment nothing moved."),
            n("Nobody won, exactly. But a crew that loses a machine a day for a season starts asking whether there is another way round."),
            n("By the first frost the stakes had been pulled and set again, further south. On the map, the line had a bend in it. A wide one."),
            c(Rabbit, "It goes around. Around the meadow. Around all of it."),
            c(Tortoise, "I have arrived. ...I gather it went well."),
            h("The hill is not coming back. I know that."),
            h("But there is a den on it again. And this time it has neighbours."),
            n("THE END"),
        ],
    }
}

/// One line for each stage's card on the map.
pub const STAGE_LINES: [&str; 18] = [
    "Ants on the north trail. Only ants, for now.",
    "Two trails, and something with wings.",
    "They are all moving with one beetle. Stop it and the rest break.",
    "A green thing that will not stay cut.",
    "The trail splits round the deadfall. So will they.",
    "The vine has found the oak.",
    "The water is low, and the mosquitoes have noticed.",
    "Two-legs, planting orange stakes.",
    "The pump truck has come for what is left of the water.",
    "Something is driving the scorpions out of the canyon.",
    "Wind, weeds and wheels.",
    "Mesa is not leaving. Mesa could not if she tried.",
    "They are cutting a track up the mountain.",
    "Machines in the air.",
    "The den under the boulder, and the thing that hunts it.",
    "Stumps as far as you can see.",
    "They have brought the sprayers.",
    "Big Yellow.",
];

/// What is said when a resident offers to come along.
pub fn join_line(who: Cr) -> &'static str {
    match who {
        Cr::Rabbit => "Sorrel will walk with you.",
        Cr::Squirrel => "Hazel is coming, asked or not.",
        Cr::Beaver => "Alder will come along.",
        Cr::Tortoise => "Mesa will come. In her own time.",
        Cr::Marmot => "Tarn is coming, to keep watch.",
        _ => "",
    }
}

/// The heading when a stage is won, or lost.
pub const WIN_WORDS: [&str; 5] = ["HELD", "THE HOME STANDS", "NOT TODAY", "STILL STANDING", "THEY TURNED BACK"];
pub const LOSE_WORDS: [&str; 3] = ["OVERRUN", "THEY GOT IN", "THE HOME IS LOST"];
