//! Words and colours: the jokes, the callouts, and the look of each chapter.

use crate::gfx::*;
use macroquad::prelude::Color;
use rufus_core::level::*;
use rufus_core::{Difficulty, Item, Rng, Special};

pub struct Theme {
    pub scene: usize,
    /// Background gradient below the scene; `top` matches the scene's ground.
    pub top: Color,
    pub bottom: Color,
}

/// Scene numbers follow the order `Gfx` loads them in, not chapter order.
pub fn theme(chapter: usize) -> Theme {
    match chapter % CHAPTERS.len() {
        0 => Theme { scene: 0, top: rgb(0x62c545), bottom: rgb(0x2a8a3c) },
        1 => Theme { scene: 1, top: rgb(0x42b8a0), bottom: rgb(0x1b7a84) },
        LAKE => Theme { scene: 5, top: rgb(0x2f9fd0), bottom: rgb(0x14507f) },
        3 => Theme { scene: 2, top: rgb(0xb07a45), bottom: rgb(0x6a4224) },
        4 => Theme { scene: 3, top: rgb(0x8c78d8), bottom: rgb(0x5540a6) },
        _ => Theme { scene: 4, top: rgb(0x2f4f8f), bottom: rgb(0x131a42) },
    }
}

/// The chapter where Rufus swims instead of runs.
pub const LAKE: usize = 2;

// Every table below has one arm per kind and then a catch-all, so a new kind
// would quietly turn up as a squeaky toy. This stops the build instead.
const _: () = assert!(rufus_core::KINDS == 7, "new piece kind: give it a sprite, colour, name, callout, quips and a cameo");
const _: () = assert!(CHAPTERS.len() == 6, "new chapter: give it a theme and a CHAPTER_DONE line");

pub fn kind_spr(kind: u8) -> Spr {
    match kind {
        BALL => spr::BALL,
        SQUIRREL => spr::SQUIRREL,
        CAT => spr::CAT,
        SOCK => spr::SOCK,
        POODLE => spr::POODLE,
        STICK => spr::STICK,
        _ => spr::TOY,
    }
}

pub fn kind_color(kind: u8) -> Color {
    match kind {
        BALL => rgb(0xc8ea24),
        SQUIRREL => rgb(0xf58f3e),
        CAT => rgb(0x7fa2ea),
        SOCK => rgb(0xf5545a),
        POODLE => rgb(0xffd6ec),
        STICK => rgb(0xdfa868),
        _ => rgb(0xdc6cf2),
    }
}

pub fn kind_name(kind: u8) -> &'static str {
    match kind {
        BALL => "tennis balls",
        SQUIRREL => "squirrels",
        CAT => "cats",
        SOCK => "socks",
        POODLE => "poodles",
        STICK => "sticks",
        _ => "squeaky toys",
    }
}

pub fn item_spr(item: Item) -> Spr {
    match item {
        Item::Normal { kind, .. } => kind_spr(kind),
        Item::Super => spr::SUPER,
        Item::Bone => spr::BONE,
        Item::Box { hp } => {
            if hp >= 2 {
                spr::BOX2
            } else {
                spr::BOX
            }
        }
    }
}

/// Shouted as chain reactions build. Index is the cascade depth.
pub fn cascade_callout(depth: u32) -> Option<&'static str> {
    Some(match depth {
        0 | 1 => return None,
        2 => "Good boy!",
        3 => "VERY good boy!",
        4 => "SUCH a good boy!",
        5 => "WHO'S A GOOD BOY?!",
        6 => "IT'S RUFUS!!",
        7 => "13/10 WOULD PET",
        _ => "BEST. BOY. EVER.",
    })
}

pub fn made_callout(item: Item) -> &'static str {
    match item {
        Item::Super => "GOLDEN RUFUS!",
        Item::Normal { special: Special::Bomb, .. } => "BIG WOOF!",
        _ => "ZOOMIES!",
    }
}

/// What gets yelled when Golden Rufus goes after one kind of thing.
pub fn fetch_callout(kind: u8) -> &'static str {
    match kind {
        BALL => "BALL! BALL! BALL!",
        SQUIRREL => "SQUIRREL!!!",
        CAT => "CHASE THE... oh. Cat.",
        SOCK => "ALL THE SOCKS!",
        POODLE => "NEW FRIENDS!",
        STICK => "EVERY. SINGLE. STICK.",
        _ => "SQUEAK SQUEAK SQUEAK!",
    }
}

/// Rufus's private thoughts when a lot of one thing goes by.
pub fn quip(kind: u8, rng: &mut Rng) -> &'static str {
    let lines: &[&str] = match kind {
        BALL => &["BALL.", "throw it. throw it. throw it.", "it's MY ball.", "again!"],
        SQUIRREL => &["SQUIRREL!!", "I KNEW IT.", "it's back. IT'S BACK."],
        CAT => &["she's judging me.", "we don't talk about the cat.", "...I'll allow it."],
        SOCK => &["mine now.", "finders keepers.", "this one's still warm."],
        POODLE => &["NEW FRIEND!", "hello. hi. hello.", "she smells AMAZING."],
        STICK => &["STICK.", "this is the best stick.", "I have a stick. look. LOOK.", "it's bigger than last time."],
        _ => &["squeak squeak.", "it's dead now. I killed it.", "the stuffing was a surprise."],
    };
    rng.pick(lines)
}

pub const IDLE_QUIPS: [&str; 8] = [
    "...snack?",
    "is it walk time?",
    "I heard a cheese wrapper.",
    "you're doing great.",
    "*heavy sigh*",
    "pet me?",
    "I could go for a nap.",
    "did you say 'park'?",
];

pub const BOOP_QUIPS: [&str; 10] = [
    "boop!",
    "hi!",
    "that's my nose.",
    "again.",
    "13/10.",
    "I'm a good boy.",
    "do you have snacks?",
    "woof.",
    "boop accepted.",
    "more please.",
];

pub const LOW_MOVES: [&str; 3] = ["uh oh.", "no pressure.", "I believe in you."];
pub const SHUFFLE_QUIPS: [&str; 3] = ["I helped!", "fixed it.", "it was boring anyway."];
pub const ZOOM_READY: [&str; 3] = ["ZOOMIES?!", "can I? CAN I?", "I've got the wiggles."];
pub const BAD_SWAP: [&str; 4] = ["?", "hm?", "that doesn't go there.", "try again?"];

/// The three difficulty settings: what the button says, and what it means.
pub fn difficulty_name(d: Difficulty) -> &'static str {
    match d {
        Difficulty::Easy => "EASY",
        Difficulty::Normal => "NORMAL",
        Difficulty::Hard => "HARD",
    }
}

pub fn difficulty_blurb(d: Difficulty) -> &'static str {
    match d {
        Difficulty::Easy => "Lap dog: plenty of moves, lots of treats.",
        Difficulty::Normal => "Good boy: a fair game of fetch.",
        Difficulty::Hard => "Top dog: every single move counts.",
    }
}

/// The pitiful look. What he says while giving it, what goes on the card
/// under his face, and what he says once it has worked (it always works).
pub const BEG_SAYS: [&str; 4] = ["...please?", "just a few more?", "*smallest whine*", "I've never been fed."];
pub const BEG_LINES: [&str; 6] = [
    "Nobody can say no to that face.",
    "He isn't even blinking.",
    "You were going to stop. Then he did the eyes.",
    "This is the look that gets him the last bite of toast.",
    "He's been practising this one in the mirror.",
    "Resistance lasted about two seconds.",
];
pub const BEG_THANKS: [&str; 4] = ["works every time.", "I KNEW it.", "you're my favourite.", "thank you thank you"];
/// Coming out of the lake, mid-shake.
pub const SHAKE_SAYS: [&str; 3] = ["*shake shake shake*", "you looked dry.", "sharing is caring."];

/// The Rufus Report: one headline per win.
pub const WIN_LINES: [&str; 36] = [
    "Rufus caught the tennis ball. He will not be giving it back.",
    "Rufus found a sock. The sock is his now.",
    "Rufus treed a squirrel. The squirrel remains unimpressed.",
    "Rufus was told he's a good boy. He already knew.",
    "Somebody said W-A-L-K. Chaos followed.",
    "Rufus ate something in the yard. Nobody knows what. Not even Rufus.",
    "Dinner is in three hours. Rufus has begun staring.",
    "The cat allowed Rufus to exist near her. Historic.",
    "Rufus shook off right next to you. You're welcome.",
    "Rufus barked at the mailman. The mailman left. Rufus remains undefeated.",
    "Rufus got a bath. He has already rolled in something.",
    "Rufus sat when asked. On the third ask. Still counts.",
    "Rufus met a new dog. They are best friends forever now.",
    "Rufus heard the treat bag from another postcode.",
    "Rufus guarded the house from a leaf. The leaf has been dealt with.",
    "Rufus was not on the couch. Please ignore the warm spot.",
    "Rufus brought you a gift. It is damp. It is a sock.",
    "Rufus did a big stretch. Everyone clapped.",
    "Rufus spun around three times before lying down. As is tradition.",
    "Rufus sneezed and scared himself.",
    "Rufus dropped the ball at your feet, then took it back. Twice.",
    "Rufus's tail has knocked over one (1) drink.",
    "Rufus found the one muddy spot in the whole park.",
    "Rufus has been fed. Rufus has never been fed in his life.",
    "Rufus dreamed about running. His feet agreed.",
    "Rufus leaned on your legs with his whole weight. It's called love.",
    "Rufus carried a stick that was much too big. He made it work.",
    "Rufus licked the dishwasher clean. He's helping.",
    "Rufus put his head on your knee and sighed. You're not going anywhere.",
    "Rufus is the goodest boy. This has now been scientifically proven.",
    "Rufus fetched the stick from the lake. He came back with a different, worse stick.",
    "Rufus swam out to the ducks. The ducks relocated. Rufus is calling that a win.",
    "Rufus is 30% lake water now. He is saving it for the car.",
    "Rufus found a stick so good he had to lie down with it.",
    "Rufus looked pitiful at a stranger with a sandwich. It worked.",
    "Rufus was asked to drop the stick. He has taken it under advisement.",
];

pub const LOSE_LINES: [&str; 12] = [
    "Rufus isn't mad. He's just going to look at you like this.",
    "It's okay. Rufus forgot what you were doing anyway.",
    "Rufus thinks you did great. Rufus also thinks a rock is food.",
    "So close! Rufus would like to try again. And a snack.",
    "Rufus has seen worse. He once lost to a screen door.",
    "No treats this time. Rufus is being very brave about it.",
    "Rufus suggests zoomies. Rufus always suggests zoomies.",
    "The squirrels win this round. THIS round.",
    "Rufus still loves you. It's sort of his whole thing.",
    "Out of moves, not out of tail wags.",
    "He did the eyes. You held firm. He is doing them harder now.",
    "Rufus is not begging. Rufus is just sitting here. Looking. At you.",
];

/// Shown on the last level's win card instead of a headline.
pub const FINALE: &str = "That's a full day of being a very good dog. Rufus is going to sleep for eleven hours. You've earned a nap too.";

/// Finishing a chapter gets its own line on the win card.
pub const CHAPTER_DONE: [&str; 6] = [
    "The backyard is secure. Three new holes have been dug. Rufus regrets nothing.",
    "Every dog at the park has been sniffed. Rufus knows things now.",
    "Rufus has been in the lake nine times. Nobody threw anything the last four. He is ready to go again.",
    "Rufus is 40% mud by weight. He would like to come inside.",
    "All parcels have been opened and inspected. None of them were for Rufus. Yet.",
    "",
];
