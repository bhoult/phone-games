//! The story, and the other words.
//!
//! The font is baked into the build as plain ASCII, so everything here
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
            n("Last spring, you had a den of your own."),
            n("Then the yellow machines came. By noon the hill was flat, and the den was a memory."),
            h(match hero {
                Fox => "I ran. I am good at running. I am not proud of it.",
                Badger => "I dug in and held on. It was not enough. Nothing I had was enough.",
                _ => "I watched all of it from the last tree. Then they took the tree.",
            }),
            n("Now the swarms are on the move. Heading west, into the meadow, eating as they go."),
            h("Somebody down there is about to lose their home."),
            h("Not this time."),
        ],
        Beat::Arrive(0) => vec![
            c(Rabbit, "Ants! Beetles! In the WARREN! Sixty cousins and nowhere to put them!"),
            c(Rabbit, "You. Stranger. You look like you bite. Do you bite?"),
            h("When it helps. Show me the trail they come in by."),
        ],
        Beat::Held(0) => vec![
            c(Rabbit, "We did it! The warren stands! Old Mandible is somebody else's problem now!"),
            h("They never stopped to eat. Did you see? That was not a raid."),
            c(Rabbit, "...They were running. From something east of here."),
            h("Then east is where I am going."),
            c(Rabbit, "Then I am coming with you. Somebody has to carry the snacks."),
        ],
        Beat::Arrive(1) => vec![
            c(Squirrel, "Halt! State your business in the Old Wood!"),
            c(Rabbit, "We are here to help! Also, hello! I am Clover!"),
            c(Squirrel, "Hmph. See that vine? It was not here last week. This morning it ate a whole beech."),
        ],
        Beat::Held(1) => vec![
            c(Squirrel, "The oak is safe. I counted the acorns. Twice."),
            c(Squirrel, "That strangling vine only grows where the ground has been cut. I followed it. It comes from the marsh side."),
            h("Cut by what?"),
            c(Squirrel, "That is what I mean to find out. Make room."),
        ],
        Beat::Arrive(2) => vec![
            c(Beaver, "Mind the mud. Used to be water. Drops a paw's width every day now."),
            c(Beaver, "And there are two-legs about. Planting orange sticks. In a line."),
            h("A line to where?"),
            c(Beaver, "Straight through my lodge, far as I can tell."),
        ],
        Beat::Held(2) => vec![
            c(Beaver, "Well. That is the pump stopped. Water will come back, given time."),
            c(Beaver, "Those orange sticks, though. I walked the line. Dead straight, east to west. Does not bend for anything."),
            h("It goes through the meadow."),
            c(Beaver, "Goes through everything. I would like a word with whoever drew it."),
        ],
        Beat::Arrive(3) => vec![
            c(Tortoise, "Visitors. ...Give me a moment. ...There."),
            c(Tortoise, "I have seen a hundred summers on this rock. Never once a line that straight."),
            c(Tortoise, "It frightens the scorpions. Everything in front of it runs. You have met them, I think."),
        ],
        Beat::Held(3) => vec![
            c(Tortoise, "Thank you. I would have managed. ...In a year or two."),
            c(Tortoise, "The line climbs from here. Up the crag. From the top you can see where it begins."),
            h("Then that is where we look."),
            c(Tortoise, "I will come. Do not wait for me. I will catch up."),
        ],
        Beat::Arrive(4) => vec![
            c(Marmot, "WHO GOES... oh. Sorry. Lookout. Habit. I am Pip."),
            c(Marmot, "They are cutting a track up my mountain. Machines in the air, too. Nobody asked me."),
            c(Marmot, "Hold the den with me and I will show you the view. You will want to see the view."),
        ],
        Beat::Held(4) => vec![
            c(Marmot, "There. Look east. No, further. See it?"),
            n("A gray stripe, ruler straight, from the edge of the town: through the desert, the marsh, the Old Wood. Aimed at the meadow like a thrown stick."),
            h("A road."),
            h("And it starts where my den used to be."),
            c(Marmot, "Then that is where it stops. I am coming. Somebody has to keep watch."),
        ],
        Beat::Arrive(_) => vec![
            n("Road's End. Stumps, mud and orange cones. Somewhere under all of it, a hill that used to be home."),
            h("This is where it started."),
            h("We dig a den. Right here. And this time we hold it."),
        ],
        Beat::Held(_) => vec![],
        Beat::Ending => vec![
            n("Big Yellow rolled to a stop. For a long moment, nothing moved."),
            n("Then a small two-legs climbed down from the cab, and looked at the den, and the pack, and the tortoise still arriving."),
            n("By next morning there was a new sign at Road's End. The line on the map had a bend in it now. A wide one."),
            c(Rabbit, "It goes AROUND! Around the meadow! Around EVERYTHING!"),
            c(Tortoise, "...I have arrived. What did I miss?"),
            h("I came looking for a home."),
            h("Turns out you build one out of neighbours."),
            n("THE END. Thank you for defending the biomes."),
        ],
    }
}

/// One line for each stage's card on the map.
pub const STAGE_LINES: [&str; 18] = [
    "Ants on the north trail. Plain ones, for now.",
    "Two trails, and something with wings.",
    "They are all following one beetle. Stop him and the rest lose heart.",
    "A green thing that will not stay shooed.",
    "The trail splits round the old log. So will they.",
    "The vine has found the oak.",
    "The water is low and the mosquitoes are thrilled.",
    "Two-legs with orange sticks.",
    "The pump truck is coming for the last of the water.",
    "Something is driving the scorpions out of the canyon.",
    "Wind, weeds and wheels.",
    "Mesa is not leaving. Mesa could not if she tried.",
    "They are cutting a track up the mountain.",
    "Machines in the air.",
    "The den under the boulder, and the thing that flies.",
    "Stumps as far as you can see.",
    "They have brought the sprayers.",
    "Big Yellow.",
];

/// What each resident says when they offer to come along.
pub fn join_line(who: Cr) -> &'static str {
    match who {
        Cr::Rabbit => "Clover wants to come with you.",
        Cr::Squirrel => "Hazel has decided she is coming with you.",
        Cr::Beaver => "Mudge would like to come along.",
        Cr::Tortoise => "Mesa will come with you. Eventually.",
        Cr::Marmot => "Pip is coming with you, to keep watch.",
        _ => "",
    }
}

/// Shouted when a stage is won.
pub const WIN_WORDS: [&str; 6] = ["HELD!", "HOME SAFE!", "SHOOED!", "NOT TODAY!", "STILL STANDING!", "DEFENDED!"];
pub const LOSE_WORDS: [&str; 3] = ["OVERRUN", "THEY GOT IN", "LOST THE DOOR"];
