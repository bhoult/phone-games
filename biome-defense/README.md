# Biome Defense

Made by Claude Fable 5.2, instigated by [Brandon Hoult](https://hoult.co).

**[Play it in your browser](https://bhoult.github.io/phone-games/biome-defense/)**, or on your phone: open that link and
add it to the home screen. After the first visit it plays with no connection.

[![The title screen, a stage in the Old Wood, the map, and a skill tree](docs/screens.jpg)](https://bhoult.github.io/phone-games/biome-defense/)

A tower defense for phones about wild creatures holding on to their homes.
You are a fox, a badger or an owl who lost a den to the yellow machines, and
you are not going to watch it happen to anyone else. Cross six biomes, help
whoever lives in each one defend their warren, oak, lodge, burrow or den from
the insects, plants and people coming up the trail, and find out what is
driving them all west.

Written in Rust, drawn with WebGL, compiled to one WebAssembly file. Everything
(art, font, sound) is inside the build: it makes no network requests and runs
offline.

## Play it

Online: <https://bhoult.github.io/phone-games/biome-defense/>

From a copy of this repository:

```
./build.sh serve
```

then open <http://localhost:8080>. Needs Rust with the `wasm32-unknown-unknown`
target (the script adds the target if it's missing) and Python 3 for the little
web server.

- **Tap a mound** to build a structure on it. Tap a structure to upgrade or
  sell it. Every biome has its own: thistles and beehives in the meadow, frogs
  and eels in the marsh, rattlesnakes in the desert, eagles on the crag.
- **Tap the ground** to send your creature there. Fighters hold invaders up
  where they stand; the owl and the other perchers strike from where nothing
  can reach them.
- **Tap the gold button** on a creature's card to use its ability: a pounce, a
  ground slam, a gust that blows the trail clear.
- **START** brings the first wave. After that they come by themselves; calling
  one early pays a few seeds.
- Invaders that reach the home cost hearts. Keep nine in ten for three stars.
- Fliers go over fighters and are only hit by things that reach the air.
  Armor shrugs off light hits. Things on wheels cannot be held up. The strip
  above the cards shows who is coming next: tap one to read about it.

**The pack.** Hold the last stage of a biome and whoever lives there offers to
come with you: Clover the rabbit, Hazel the squirrel, Mudge the beaver, Mesa
the tortoise, Pip the marmot. Two can walk with you at a time. The others wait
at home, and fight for it when you pass through.

**Skills.** Every stage held for the first time earns one skill point and one
bond point. Your own creature spends skill points on its three branches. Each
skill costs one more than the one before it, and there are 18 points in the
whole game against 30 to learn everything, so reaching the end of a branch
means going without elsewhere, and what you choose is for keeps. Followers
learn with bond points, shared between them; a follower who goes home hands
theirs back for whoever walks with you next. All of it is under **PACK** on
the map.

**Difficulty** is a button on each stage's card: Easy, Normal or Hard. It
changes how tough invaders are and how many hearts the home has; the stages
and the points are the same. Lose a stage and the card offers to drop down a
setting with one tap.

Progress is saved on the device.

## Putting it on a phone

Open <https://bhoult.github.io/phone-games/biome-defense/> on the phone and
choose "Add to Home screen" (Android Chrome: the menu; iPhone Safari: the share
button). A service worker caches all nine files of the game on the first visit,
so from then on it starts from the home screen and plays with no connection.

`./build.sh` leaves the whole game in `dist/` as nine static files that can go
on any web host. One catch: browsers only allow the offline caching on
**https** or **localhost**.

The copy at the link above is `docs/biome-defense` at the top of this
repository, served by GitHub Pages. `./build.sh pages` rebuilds it; commit and
push to publish.

There is no Android app for this one yet (Rufus Crush next door has one; the
same wrapper would fit).

## How it's put together

```
core/     the rules. Pure Rust, no dependencies, no rendering.
  src/sim.rs        a stage being played, in fixed ticks: invaders, structures, the pack
  src/defs.rs       the invaders, the structures and the biomes, as data
  src/creature.rs   the eight creatures, their abilities and skill trees
  src/level.rs      the 18 stages: maps, who comes, and the wave generator
  src/campaign.rs   the journey: stars, points, who is in the pack, the save format
  src/bot.rs        a stand-in player for testing and tuning
  tests/rules.rs    rule tests, whole stages, a random-tapping player
  examples/balance.rs   reports on, and sets, how hard the stages are
game/     everything you see and hear (macroquad -> WebGL).
  src/play.rs       a stage: steps the rules and shows what they report
  src/field.rs      the ground, trail and scenery of a stage
  src/map.rs        the trail through the biomes
  src/pack.rs       the pack and the skill trees
  src/story.rs      the story, and who joins or stays behind
  src/content.rs    the words
  src/synth.rs      every sound and both tunes, synthesised at startup
  assets/svg/       all the art, as SVG
  build.rs          rasterises the SVGs into a texture atlas, and the font into a
                    texture of glyphs, at build time
web/      index.html, the JS bridge, service worker, manifest
tools/    test driver, contact sheets, balance tuning
```

The rules never animate anything. They advance in ticks of a sixtieth of a
second and report what happened (a structure fired, an invader was shooed, a
dam went up); the game layer turns that into pictures and sound and draws
positions in between ticks. That split is what lets whole stages be played
natively thousands of times a second, which is how they are tested and how
their difficulty is set.

## Changing things

**Stages** are in `core/src/level.rs`: a name, a tip, the map drawn as 13 rows
of 9 characters (`.` ground, `#` trail, `o` a mound, `A` `B` where invaders
come in, `H` the home, `~` water, `T` `r` scenery), who comes and from which
wave, and how many waves. The waves themselves are generated from those. After
changing a stage, or anything about structures, invaders or creatures, run

```
tools/apply_tune.py
```

It plays every stage with the stand-in player, sizes each wave to a share of
the most the stand-in can stop, sizes each boss the same way, and writes the
numbers back into the file. It takes about half a minute.
`cargo run -p biome-core --release --example balance` prints how the stand-in
then does on Easy, Normal and Hard with each starting creature (`balance
waves` lists who comes when, `balance why 8 hard` says who got through).

**Difficulty** is one table: `CURVE` at the top of `core/examples/balance.rs`.
For each biome it gives the pressure of a wave at the start of a stage and at
the end of it, on Normal, as a share of the stand-in's limit. Raise a number to
make that stretch harder, then run `tools/apply_tune.py` again. Easy and Hard
are fixed steps either side (`Difficulty::toughness` in `core/src/level.rs`).
The numbers describe a stand-in player, not a person: if the game feels wrong
to you, move them.

**Structures, invaders and creatures** are tables in `core/src/defs.rs` and
`core/src/creature.rs`. A skill is a name, a sentence and a list of effects.

**Art** is SVG in `game/assets/svg/`. Edit a file and rebuild.
`tools/preview.py out.png game/assets/svg/atlas/enemies` renders a contact
sheet straight from the files; `tools/sheet.py out.png` renders one from the
built atlas. Every sprite is drawn in a 100 x 100 box with no ground shadow
(the game draws those).

**The story** is in `game/src/content.rs`. The font is baked into the build as
plain ASCII (anything else shows as a question mark), so keep to it.

## Testing

```
cargo test -p biome-core --release
```

runs the rule tests: every map is playable, every skill does something, points
always add up whatever is learned, swapped or sent home, doing nothing loses
every stage, the stand-in wins every stage on Easy and Normal, and a player who
taps at random for six minutes never breaks anything. `cargo test -p
biome-defense` checks every sound is audible and unclipped.

`tools/drive.mjs` drives a real build in headless Chrome at phone size, with
real touch events:

```
node tools/drive.mjs "http://localhost:8080/?debug=1&hero=fox&level=1" shots \
    wait:2500 tapmound:2 tapbtn:build0 tapbtn:call waitfor:"WAVE 2" shot:after
```

The page takes a few switches in its address for this. A session that fakes
progress or settings (`hero`, `unlock`, `fresh`, `mute`, `difficulty`) never
writes to saved data:

| switch | effect |
| --- | --- |
| `level=7` | start in that stage |
| `screen=map` | start on the map (`pack`, `choose`) |
| `hero=owl` | play as the owl (`fox`, `badger`) |
| `unlock=9` | as if the first nine stages were behind you |
| `story=1` | with `unlock`: tell the story from there, rather than skip it |
| `fresh=1` | ignore the saved journey for this session |
| `auto=1` | the stand-in plays (`auto=2` also moves on when a stage ends) |
| `speed=3` | run time faster |
| `seed=42` | same dice every time |
| `debug=1` | narrate to the console, and report where buttons and mounds are |
| `mute=1` | start silent |
| `difficulty=0` | Easy (`1` Normal, `2` Hard) |

## Updating a copy that is already installed

Reload it, wait a few seconds, and reload it again (on a phone, close it and
open it twice). The first reload fetches the new build in the background while
still showing the old one; the second one runs it. Progress is kept.

## Credits

- Font: [Lilita One](https://fonts.google.com/specimen/Lilita+One) by Juan
  Montoreano, SIL Open Font License (`game/assets/font/OFL.txt`).
- Engine: [macroquad](https://macroquad.rs) (MIT / Apache-2.0).

The game's own code and art are under the MIT licence: see `LICENSE` at the top
of this repository.
