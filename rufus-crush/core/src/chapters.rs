//! The chapters: six levels each, one after another down the map.
//!
//! The first six are Rufus's first day and were built by hand. After that
//! every day goes the same way (yard, park, water, mud, indoors, night), so
//! each later chapter is a new name on one of those six places. The very
//! last chapter is a short one: bedtime.

/// Where a chapter happens. Decides its backdrop, its colours and which
/// obstacles its levels lean on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Place {
    Yard,
    Park,
    Lake,
    Mud,
    Home,
    Night,
}

impl Place {
    pub const ALL: [Place; 6] = [Place::Yard, Place::Park, Place::Lake, Place::Mud, Place::Home, Place::Night];

    pub const fn index(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Chapter {
    pub name: &'static str,
    pub place: Place,
}

pub const CHAPTER_LEN: usize = 6;
/// Chapters in a day: one for each place.
pub const DAY_LEN: usize = 6;

const FIRST_DAY: [&str; 6] = ["The Backyard", "The Dog Park", "The Lake", "Mud Season", "Delivery Day", "Night Patrol"];

// One name per later day, for each place. Keep them under about 22
// characters: they have to fit on the ribbon across the chapter banner.
const YARD: [&str; 28] = [
    "The Front Yard",
    "The Hole (Ongoing)",
    "Sprinkler Season",
    "The Neighbour's Fence",
    "The Vegetable Patch",
    "Squirrel Country",
    "The Compost Heap",
    "Under the Deck",
    "The Hose Incident",
    "Lawnmower Day",
    "The Bird Feeder",
    "The Former Flower Bed",
    "Barbecue Watch",
    "The Garden Gate",
    "The Sunny Spot",
    "Leaf Pile Season",
    "The Back Step",
    "The Shed",
    "The Hedge Has Eyes",
    "The Washing Line",
    "Dandelion Patrol",
    "The Wheelbarrow",
    "Second Hole, Same Yard",
    "The Apple Tree",
    "The Patio",
    "Fence Line Patrol",
    "The Tomato Thief",
    "The Whole Yard, Again",
];

const PARK: [&str; 28] = [
    "The Big Field",
    "Off-Leash Hour",
    "The Other Dog Park",
    "Picnic Reconnaissance",
    "The Frisbee Question",
    "Bench Inspection",
    "The Jogging Path",
    "Puppy Class Reunion",
    "The Bandstand",
    "Greyhound Day",
    "The Tennis Courts",
    "The Long Walk",
    "The Short Walk",
    "The Farmers' Market",
    "The Playground Fence",
    "Poodle Parade",
    "The Hill",
    "The Ice Cream Van",
    "The Stick Library",
    "The Corner Shop Wait",
    "Unofficial Dog Show",
    "The Football Pitch",
    "The Woods Behind",
    "The Bin Circuit",
    "New Leash Day",
    "The Kite Flyers",
    "The Cafe Patio",
    "Everyone Is Here",
];

const LAKE: [&str; 28] = [
    "The Creek",
    "The Beach",
    "The Geese Are Back",
    "Dock Diving",
    "The Paddling Pool",
    "The Boat Ramp",
    "Low Tide",
    "The Fishing Trip",
    "The Reservoir",
    "The River Bend",
    "Canoe (Briefly)",
    "The Stepping Stones",
    "The Swimming Hole",
    "The Sandbar",
    "Seaweed Collection",
    "The Pier",
    "The Waterfall",
    "The Flooded Field",
    "The Harbour",
    "Driftwood Season",
    "The Mill Pond",
    "The Rock Pools",
    "Ferry Crossing",
    "The Deep End",
    "The Marsh",
    "The Jetty",
    "The Far Shore",
    "One Last Swim",
];

const MUD: [&str; 28] = [
    "Rainy Day",
    "The Trail",
    "Spring Thaw",
    "The Ditch",
    "After the Storm",
    "The Building Site",
    "The Farm Visit",
    "The Riverbank",
    "Pig Appreciation Day",
    "The Shortcut",
    "The Allotment",
    "The Boot Room",
    "The Bog",
    "The Tractor Path",
    "Puddle Season",
    "The Clean Car",
    "White Carpet Day",
    "The Towel Negotiation",
    "The Cow Field",
    "Garden Makeover",
    "The Long Way Home",
    "Slip and Slide",
    "The Stream Crossing",
    "Monsoon",
    "The Quarry",
    "The Hosing Down",
    "Paw Print Gallery",
    "Mud: The Sequel",
];

const HOME: [&str; 27] = [
    "The Kitchen",
    "Laundry Day",
    "Guests Are Coming",
    "The Forbidden Couch",
    "Vacuum Day",
    "The Pantry Door",
    "Moving Boxes",
    "The Hallway",
    "Grocery Delivery",
    "The Bathroom Bin",
    "The Home Office",
    "Birthday Party",
    "The Sock Drawer",
    "Pizza Night",
    "The Spare Room",
    "The Dishwasher",
    "Furniture Assembly",
    "The Stairs",
    "The Shoe Rack",
    "Holiday Decorations",
    "The Laundry Basket",
    "Sunday Roast",
    "The Garage",
    "Under the Sink",
    "Gift Wrapping",
    "The Toy Basket",
    "Spring Cleaning",
];

const NIGHT: [&str; 27] = [
    "The Thunderstorm",
    "Fireworks Night",
    "The 3 A.M. Bark",
    "Campfire",
    "The Night Walk",
    "Movie Night",
    "The Fox in the Garden",
    "Full Moon",
    "The Power Cut",
    "Sleepover",
    "The Hedgehog",
    "Midnight Kitchen",
    "The Owl",
    "Stargazing",
    "The Bed (All of It)",
    "Night Shift",
    "The Creaky Stair",
    "Something in the Bins",
    "The Late Guest",
    "Bonfire",
    "The Tent",
    "Shooting Stars",
    "The Moth",
    "The Long Night",
    "Sleepwalk Zoomies",
    "The Bat",
    "Before Dawn",
];

/// The short last chapter.
const FINALE: &str = "Bedtime, For Real";

/// Whole later days, then one more that stops after the mud and goes to bed.
const FULL_DAYS: usize = 27;
pub const CHAPTER_COUNT: usize = DAY_LEN + FULL_DAYS * DAY_LEN + 5;

pub static CHAPTERS: [Chapter; CHAPTER_COUNT] = build();

const fn build() -> [Chapter; CHAPTER_COUNT] {
    let mut out = [Chapter { name: "", place: Place::Yard }; CHAPTER_COUNT];
    let mut i = 0;
    while i < DAY_LEN {
        out[i] = Chapter { name: FIRST_DAY[i], place: Place::ALL[i] };
        i += 1;
    }
    let mut day = 0;
    while day <= FULL_DAYS {
        let at = DAY_LEN + day * DAY_LEN;
        out[at] = Chapter { name: YARD[day], place: Place::Yard };
        out[at + 1] = Chapter { name: PARK[day], place: Place::Park };
        out[at + 2] = Chapter { name: LAKE[day], place: Place::Lake };
        out[at + 3] = Chapter { name: MUD[day], place: Place::Mud };
        if day < FULL_DAYS {
            out[at + 4] = Chapter { name: HOME[day], place: Place::Home };
            out[at + 5] = Chapter { name: NIGHT[day], place: Place::Night };
        } else {
            out[at + 4] = Chapter { name: FINALE, place: Place::Night };
        }
        day += 1;
    }
    out
}

/// Which day of Rufus's life a chapter belongs to, counting from 1.
pub fn day_of(chapter: usize) -> usize {
    chapter / DAY_LEN + 1
}
