"""Every generated picture in the game: what to ask the image model for, and
how the result is cut, turned and sized for the game.

Each entry: name, group, kind, subject and options.

  kind "high"   an animal or person from 45 degrees above, facing right
  kind "top"    seen from straight above (insects, machines, fliers); the
                game turns these to face the way they are going
  kind "thing"  something standing on the ground, from 45 degrees above
  kind "face"   a portrait
  kind "tex"    a surface seen from straight above, filling the frame
  kind "scene"  a whole picture; the subject is the entire prompt

Options:
  px     longest side in the game's atlas (cut-outs), or output width (others)
  w, h   size to generate at
  seed   change it to roll again
  rot    degrees to turn the cut-out so that a "top" sprite points right
  flip   mirror the cut-out
  matte  which cut-out model to use, if not the usual one for its kind
"""

BG = "isolated on a plain flat solid mid-gray background, no ground, no cast shadow, no text, no logos, no brand markings"
PHOTO = "Photorealistic photograph, soft even daylight, sharp focus, highly detailed"


def high(subject):
    return f"{subject}, whole body in frame, high-angle view from above at 45 degrees, facing right. {PHOTO}. Single subject {BG}."


def top(subject, front="head"):
    return f"Aerial top-down view looking straight down at {subject}, seen directly from above, whole thing in frame, {front} pointing to the right. {PHOTO}. Single subject {BG}."


def thing(subject):
    return (
        f"{subject}. The entire object is inside the frame with empty margin on every side, nothing cut off by the edge of the picture. "
        f"High-angle view from above at 45 degrees. {PHOTO}. Single object {BG}."
    )


def face(subject, behind):
    return (
        f"Close-up wildlife portrait of {subject}, head and shoulders, three-quarter face, {behind} out of focus behind. "
        "Photorealistic, dramatic low-key side lighting, shallow depth of field, muted natural colours, highly detailed."
    )


def tex(subject):
    return (
        f"Top-down aerial view looking straight down at {subject}, filling the whole frame edge to edge, even soft overcast light, "
        "no horizon, no sky, no large shadows, no people, no animals. Photorealistic terrain photograph, muted natural colours, high detail."
    )


KINDS = {"high": high, "top": top, "thing": thing, "tex": tex}

ART = []


def add(name, group, kind, subject, **opt):
    ART.append(dict(name=name, group=group, kind=kind, subject=subject, **opt))


# ---- the pack, on the field -------------------------------------------------
add("fox", "creatures", "high", "A lean red fox prowling, head low", px=200, seed=22)
add("badger", "creatures", "high", "A heavy European badger walking, black and white striped face", px=200, seed=31)
add("owl", "creatures", "top", "a tawny eagle owl in flight with wings fully spread", px=230, seed=32)
add("rabbit", "creatures", "high", "A wild brown hare sitting up alert, ears raised", px=170, seed=33)
add("squirrel", "creatures", "high", "A red squirrel crouched holding an acorn, bushy tail curled up", px=160, seed=34)
add("beaver", "creatures", "high", "A wet brown beaver walking on land, flat scaly tail visible", px=190, seed=35)
add("tortoise", "creatures", "high", "A weathered desert tortoise walking, domed scarred shell", px=190, seed=36)
add("tortoise_shell", "creatures", "thing", "A closed empty-looking tortoise shell lying on the ground, only the domed scarred shell, no head, no legs, openings dark", px=170, seed=237)
add("marmot", "creatures", "high", "An alpine marmot standing upright on its hind legs keeping watch", px=170, seed=38)
add("cousin", "creatures", "high", "A small wild rabbit running flat out", px=120, seed=39)
add("decoy", "creatures", "high", "A crude fox-shaped decoy made of bundled dry grass, twigs and red leaves", px=170, seed=40)

# ---- portraits ---------------------------------------------------------------
P = dict(w=768, h=768, px=256)
add("face_fox", "faces", "scene", face("a lean red fox with an old scar across the muzzle, wary amber eyes", "a dark forest at dusk"), seed=16, **P)
add("face_badger", "faces", "scene", face("an old European badger with a grizzled black and white striped face, heavy and unimpressed", "a dark earth bank and roots"), seed=41, **P)
add("face_owl", "faces", "scene", face("a tawny eagle owl with huge orange eyes staring straight at the camera", "a night forest"), seed=42, **P)
add("face_rabbit", "faces", "scene", face("a wild brown hare, ears up, one ear notched, watchful dark eye", "a meadow at dawn"), seed=43, **P)
add("face_squirrel", "faces", "scene", face("a red squirrel with tufted ears, sharp and suspicious", "an old oak trunk"), seed=44, **P)
add("face_beaver", "faces", "scene", face("a wet brown beaver with orange teeth just showing, calm and tired", "grey marsh water and reeds"), seed=45, **P)
add("face_tortoise", "faces", "scene", face("an ancient desert tortoise with deeply wrinkled skin and a hooded patient eye", "red sandstone"), seed=46, **P)
add("face_marmot", "faces", "scene", face("an alpine marmot standing alert, frost on its whiskers", "grey scree and snow"), seed=47, **P)

# the three you can play as, full figure, for the choosing screen
T = dict(w=768, h=1024, px=512)
add("hero_fox", "scenes", "scene", "A lean scarred red fox standing on a fallen log in a dark forest at dusk, looking toward the camera, full body, cold blue shadows and one warm rim of last light, mist. Photorealistic wildlife photograph, cinematic, moody, muted colours, vertical composition.", seed=48, **T)
add("hero_badger", "scenes", "scene", "A powerful old European badger standing squarely at the mouth of a den among tree roots at dusk, facing the camera, full body, earth on its claws, cold blue shadows and one warm rim of last light. Photorealistic wildlife photograph, cinematic, moody, muted colours, vertical composition.", seed=49, **T)
add("hero_owl", "scenes", "scene", "A tawny eagle owl perched on a dead branch at night, huge orange eyes looking at the camera, full body, moonlit rim light, dark forest behind, mist. Photorealistic wildlife photograph, cinematic, moody, muted colours, vertical composition.", seed=50, **T)

# ---- invaders ---------------------------------------------------------------
E = "enemies"
add("ant", E, "top", "a single black garden ant, six legs spread", px=110, seed=25)
add("beetle", E, "top", "a single heavy black ground beetle with a glossy ridged armoured shell, legs spread", px=140, seed=51, rot=-90)
add("locust", E, "top", "a single brown desert locust with long folded hind legs", px=120, seed=52, rot=-90)
add("wasp", E, "top", "a single yellow and black wasp flying with wings spread", px=120, seed=53)
add("moth", E, "top", "a single large grey-brown hawk moth with wings spread open", px=130, seed=54, rot=-90)
add("mosquito", E, "top", "a single mosquito with long thin legs spread and wings out", px=96, seed=55)
add("slug", E, "top", "a single large fat brown-orange slug, glistening wet", px=150, seed=56, rot=-90)
add("fire_ant", E, "top", "a single red fire ant, six legs spread, jaws open", px=110, seed=57, rot=-90)
add("scorpion", E, "top", "a single large dark scorpion with claws forward and tail curled over its back", px=160, seed=58, rot=90)
add("creeper", E, "top", "a crawling tangled mass of green kudzu vine with reaching tendrils and leaves, low and spreading", front="the longest tendrils", px=140, seed=59)
add("spore_pod", E, "top", "a tangled clump of thick vine carrying three swollen pale green seed pods about to burst", front="the pods", px=150, seed=60)
add("sprout", E, "top", "a single small pale green vine shoot with a curled tip, two leaves and trailing white roots", front="the curled tip", px=96, seed=61)
add("tumbleweed", E, "top", "a single dry round tumbleweed, a ball of grey-brown tangled thorny twigs", front="it", px=140, seed=62)
add("surveyor", E, "high", "A land surveyor in an orange high-visibility vest and white hard hat walking to the right, seen from the side, carrying a yellow surveying tripod over one shoulder and a bundle of orange wooden stakes in the other hand", px=180, seed=214)
add("camper", E, "high", "A hiker walking with a very large backpack, rolled tent and trekking poles, beanie hat, checked shirt", px=180, seed=63)
add("trapper", E, "high", "A big bearded trapper in a worn waxed jacket and heavy boots walking, steel traps and chains hanging from a heavy pack, an axe in one hand", px=190, seed=64)
add("worker", E, "high", "A construction worker in a yellow hard hat and orange high-visibility vest walking, carrying a chainsaw", px=180, seed=24)
add("exterminator", E, "high", "A pest control worker in a white hooded coverall and respirator mask walking, with a backpack sprayer tank and a spray wand held out in front", px=190, seed=65)
add("atv", E, "top", "a muddy quad bike with a rider in a dark helmet", front="the front of the quad bike", px=170, seed=26, rot=90)
add("mower", E, "top", "a ride-on brush mower with a wide steel cutting deck at the front and an operator in ear defenders", front="the cutting deck", px=180, seed=66, rot=-90)
add("drone", E, "top", "a grey quadcopter camera drone with four spinning rotors", front="the camera", px=130, seed=67, rot=90)
add("stag_beetle", E, "top", "a single enormous stag beetle with huge branching antler jaws, glossy dark red-black shell, scarred", px=300, seed=12, rot=-90)
add("strangler", E, "top", "a huge knotted heap of thick brown woody strangler vines covered in glossy dark green leaves, full colour, with long green tendrils reaching out to one side", front="the longest tendrils", px=330, seed=268)
add("pump_truck", E, "top", "the roof of a dirty water tanker truck, one rigid vehicle: the cab roof at one end and a long cylindrical steel tank with hatches and coiled hoses along the rest, photographed by a drone hovering exactly overhead so that no side of the truck is visible", front="the cab", px=340, seed=369)
add("buggy", E, "top", "a stripped-down desert dune buggy with a roll cage, big knobbly tyres and a helmeted driver, covered in red dust", front="the front of the buggy", px=320, seed=70, rot=90)
add("chopper", E, "top", "a dark grey utility helicopter with a long tail boom and blurred spinning rotor disc", front="the nose", px=360, seed=71, rot=90)
add("big_yellow", E, "top", "a massive yellow bulldozer with a wide steel blade at the front, mud-caked tracks and an enclosed cab", front="the blade", px=400, seed=72, rot=90)

# ---- structures -------------------------------------------------------------
S = "towers"
add("shrike", S, "thing", "A dense thorny hawthorn bush with a great grey shrike perched on its highest twig, growing from a clump of earth", px=200, seed=73)
add("puffball", S, "thing", "A tight cluster of large white giant puffball mushrooms growing from a mossy clump of earth, a wisp of brown spores", px=180, seed=74)
add("bramble", S, "thing", "A low wide tangled thicket of bramble with hooked thorns, glossy dark green leaves and a few ripe blackberries, full colour, sprawling over the ground", px=200, seed=275)
add("hive", S, "thing", "A wild honeybee colony in an upright hollow log section standing on the ground, golden comb showing in a split, bees crawling over it and flying around", px=190, seed=76)
add("rookery", S, "thing", "A short gnarled dead tree stump, roots and base visible, with a big untidy stick nest on top and two black crows, one holding a stone in its beak", px=210, seed=277)
add("bat_roost", S, "thing", "A tall hollow dead tree trunk broken off at the top, with small dark bats flying out of a hole in it", px=210, seed=78)
add("sap_run", S, "thing", "A scarred pine stump bleeding thick amber resin that runs down and pools on the ground around it", px=180, seed=79)
add("woodpecker", S, "thing", "A dead tree snag full of holes, broken off at the top, roots and base visible, with a black woodpecker with a red crown clinging to its side", px=210, seed=280)
add("kingfisher", S, "thing", "A clump of tall cattail reeds growing from mud with a bright blue and orange kingfisher perched on a bent reed", px=200, seed=81)
add("bullfrog", S, "thing", "A small round muddy pool edged with moss and stones, with a very large green bullfrog sitting at the edge", px=180, seed=82)
add("mud_wallow", S, "thing", "A round churned wallow of deep wet dark mud with glistening puddles", px=180, seed=83)
add("eel_pool", S, "thing", "A small round pool of dark water edged with wet stones, with a thick dark eel coiled just under the surface", px=180, seed=84)
add("cholla", S, "thing", "A single jumping cholla cactus plant, densely covered in pale barbed spines, growing from a small clump of dry stony earth", px=200, seed=285)
add("bighorn", S, "thing", "A bighorn ram with heavy curled horns standing on top of a small compact pile of red sandstone boulders", px=220, seed=286)
add("dust_devil", S, "thing", "A small ring of wind-scoured red sandstone rocks around a patch of bare sand, with dust lifting from it", px=180, seed=87)
add("rattler_den", S, "thing", "A western diamondback rattlesnake coiled on a small pile of flat red rocks, triangular head raised, no hood, diamond pattern on its back, the rattle at the tip of its tail held up", px=190, seed=288)
add("icefall", S, "thing", "A grey boulder resting on the ground with a row of long sharp icicles hanging from its overhanging upper edge, frost on the rock", px=200, seed=289)
add("cornice", S, "thing", "A tall rounded heap of packed snow piled over grey rocks, resting on the ground, with a heavy overhanging lip of snow", px=200, seed=290)
add("frost_vent", S, "thing", "A dark crack in grey frost-covered rock rimmed with ice crystals, cold white mist pouring out of it", px=180, seed=91)
add("eyrie", S, "thing", "A golden eagle standing on a huge nest of sticks built on top of a short grey rock pinnacle whose base rests on the ground", px=220, seed=292)

# ---- homes ------------------------------------------------------------------
W = "world"
add("home_warren", W, "thing", "A grassy earth bank with several dark rabbit burrow entrances worn into it, bare soil at the mouths", px=260, seed=93)
add("home_oak", W, "thing", "A whole ancient oak tree with a massive gnarled trunk, a dark hollow at its base between thick roots, and a dense broad canopy", px=280, seed=294)
add("home_lodge", W, "thing", "A beaver lodge, a large dome of piled sticks, branches and mud", px=260, seed=95)
add("home_burrow", W, "thing", "A dark burrow entrance dug under an overhanging slab of red sandstone, a fan of dug sand in front", px=260, seed=96)
add("home_den", W, "thing", "A dark den entrance under a huge grey granite boulder with patches of snow and alpine grass", px=260, seed=97)
add("home_roadsend", W, "thing", "A freshly dug den entrance in a mound of raw earth between two sawn tree stumps, a few green shoots growing from the soil", px=260, seed=98)
add("dam", W, "top", "a beaver dam, a thick barrier of piled sticks, branches and wet mud", front="one end", px=200, seed=99)

# ---- scenery ----------------------------------------------------------------
add("deco_meadow_tall", W, "thing", "A small hawthorn tree in leaf with a twisted trunk", px=230, seed=100)
add("deco_meadow_rock", W, "thing", "A lichen-covered grey boulder half sunk in grass", px=150, seed=101)
add("deco_forest_tall", W, "thing", "An old oak tree with a dense dark green canopy", px=250, seed=29)
add("deco_forest_rock", W, "thing", "A mossy rotting fallen log with bracket fungus", px=170, seed=102)
add("deco_marsh_tall", W, "thing", "A small pollarded willow tree with drooping grey-green leaves", px=230, seed=103)
add("deco_marsh_rock", W, "thing", "A dense tussock of sedge grass on a muddy mound", px=150, seed=104)
add("deco_desert_tall", W, "thing", "A tall saguaro cactus with two arms", px=230, seed=105)
add("deco_desert_rock", W, "thing", "A weathered red sandstone boulder", px=150, seed=106)
add("deco_alpine_tall", W, "thing", "A stunted wind-bent mountain pine with snow on its branches", px=230, seed=107)
add("deco_alpine_rock", W, "thing", "A grey granite boulder with a cap of snow", px=150, seed=108)
add("deco_road_tall", W, "thing", "A freshly sawn tree stump with sawdust and an orange survey stake hammered in beside it", px=170, seed=109)
add("deco_road_rock", W, "thing", "A pile of broken concrete rubble and a fallen orange traffic cone", px=160, seed=110)

# ---- things in flight -------------------------------------------------------
F = "shots"
add("shot_shrike", F, "top", "a great grey shrike in flight with wings spread", px=72, seed=111)
add("shot_kingfisher", F, "top", "a bright blue kingfisher in flight with wings spread", px=72, seed=112, rot=-40)
add("shot_bat", F, "top", "a small dark bat in flight with wings spread", px=64, seed=113, rot=-90)
add("shot_eagle", F, "top", "a golden eagle in flight with wings fully spread", px=150, seed=114)
add("shot_woodpecker", F, "top", "a black woodpecker with a red crown in flight with wings spread", px=80, seed=115)
add("shot_stone", F, "thing", "A single rough grey stone the size of a fist", px=44, seed=116)
add("shot_boulder", F, "thing", "A single round red sandstone boulder", px=72, seed=117)
add("shot_snow", F, "thing", "A single rough lump of packed snow", px=64, seed=118)
add("shot_icicle", F, "top", "a single long sharp icicle", front="the sharp tip", px=64, seed=119, rot=90)
add("shot_acorn", F, "thing", "A single acorn with its cap", px=40, seed=120)

# ---- surfaces ---------------------------------------------------------------
G = dict(w=832, h=1248, px=768)
X = "textures"
add("ground_meadow", X, "tex", "wild meadow ground: short grass, clover, patches of bare earth, a few small white and yellow wildflowers and pebbles", seed=15, **G)
add("ground_forest", X, "tex", "old woodland floor: dark leaf litter, moss, ferns, fallen twigs, small roots and patches of bare damp soil", seed=121, **G)
add("ground_marsh", X, "tex", "marshland: wet dark mud, mats of green moss and sedge, flattened reeds, small shallow puddles", seed=122, **G)
add("ground_desert", X, "tex", "red desert ground: dry cracked red earth, sand, scattered small stones and a few dry grey scrub plants", seed=123, **G)
add("ground_alpine", X, "tex", "high mountain ground: patchy snow over grey scree, lichen-covered stones and tufts of frozen brown grass", seed=124, **G)
add("ground_road", X, "tex", "the ground of a clear-cut forest: churned brown mud, wood chips, sawdust, bark, crushed branches and small puddles scattered evenly, no tracks, no ruts, no road", seed=325, **G)
D = dict(w=768, h=768, px=512)
add("trail_meadow", X, "tex", "a bare trodden dirt path surface: packed brown earth, small stones, scuffed and dry, no grass", seed=28, **D)
add("trail_forest", X, "tex", "a forest path surface: dark packed damp soil with trodden leaf fragments, small roots and stones, no plants", seed=126, **D)
add("trail_marsh", X, "tex", "a muddy track surface: wet grey-brown churned mud with footprints and small puddles, no plants", seed=127, **D)
add("trail_desert", X, "tex", "a desert track surface: pale packed sand and fine gravel with faint tyre marks, no plants", seed=128, **D)
add("trail_alpine", X, "tex", "a mountain path surface: trampled dirty snow and dark wet gravel, no plants", seed=129, **D)
add("trail_road", X, "tex", "a rough construction haul road surface: compacted grey crushed stone and gravel with tyre tracks, no plants", seed=130, **D)
add("water", X, "tex", "still dark pond water with faint ripples, a few floating leaves and green duckweed", seed=131, **D)

# ---- whole pictures ---------------------------------------------------------
C = "scenes"
V = dict(w=704, h=1520, px=640)
add("title", C, "scene", "A lone red fox standing on a rocky ridge at dusk, seen from behind in the lower third of the frame, looking out over a dark forested valley. Far below, a raw straight road has been cut through the forest, with tiny distant yellow machines and work lights and a haze of dust. Heavy brooding sky filling the upper half, last orange light on the horizon. Photorealistic, cinematic, sombre, muted colours, vertical composition.", seed=132, **V)
L = dict(w=768, h=1360, px=640)
add("story_intro", C, "scene", "A bulldozed hillside at grey dawn: raw churned earth, deep caterpillar tracks, torn roots and a toppled tree where an animal den used to be, a distant yellow bulldozer in the mist. Photorealistic, cinematic, sombre, desaturated, vertical composition, no people.", seed=133, **L)
add("story_meadow", C, "scene", "A wide wild meadow of long grass and wildflowers at first light under a heavy overcast sky, a low earth bank with rabbit burrows in the middle distance, mist lying in the hollows. Photorealistic landscape photograph, cinematic, quiet, muted colours, vertical composition, no people.", seed=134, **L)
add("story_forest", C, "scene", "Inside an ancient oak wood, huge mossy trunks, shafts of dim light through mist, a thick vine strangling one tree. Photorealistic landscape photograph, cinematic, quiet, muted green and brown, vertical composition, no people.", seed=135, **L)
add("story_marsh", C, "scene", "A grey marsh at dusk, low water between reed beds and exposed cracked mud, a beaver lodge in the middle distance, a line of orange survey stakes running straight across the mud. Photorealistic landscape photograph, cinematic, quiet, muted colours, vertical composition, no people.", seed=136, **L)
add("story_desert", C, "scene", "A red rock desert canyon in harsh late light, sandstone walls and dry scrub, a dust plume rising far off, long shadows. Photorealistic landscape photograph, cinematic, quiet, muted red and ochre, vertical composition, no people.", seed=137, **L)
add("story_alpine", C, "scene", "A high mountain crag above the tree line at cold blue dawn, snow patches, grey boulders and scree, a raw track cut zigzagging up the slope below, cloud in the valley. Photorealistic landscape photograph, cinematic, quiet, muted blue-grey, vertical composition, no people.", seed=138, **L)
add("story_road", C, "scene", "The end of an unfinished road through a clear-cut forest at dusk: tree stumps, mud, orange traffic cones and a parked yellow bulldozer with its lights on, smoke haze, dark sky. Photorealistic, cinematic, sombre, muted colours, vertical composition, no people.", seed=139, **L)
add("story_ending", C, "scene", "First light over a clear-cut hillside where a silent yellow bulldozer stands abandoned, and in the foreground green shoots and wildflowers growing back through the churned earth, mist lifting, pale gold sky. Photorealistic, cinematic, hopeful but quiet, muted colours, vertical composition, no people.", seed=140, **L)
# the map: each biome from high above, top to bottom of one long scroll
M = dict(w=1024, h=1024, px=640)
add("map_meadow", C, "scene", "High aerial photograph looking straight down from far above at rolling wild meadow, hedgerows and a small stream, soft overcast light, muted greens, no buildings, no roads.", seed=141, **M)
add("map_forest", C, "scene", "High aerial photograph looking straight down from far above at dense ancient broadleaf forest canopy with a few small clearings, soft overcast light, deep muted greens, no buildings, no roads.", seed=142, **M)
add("map_marsh", C, "scene", "High aerial photograph looking straight down from far above at marshland: winding dark water channels, reed beds and mudflats, soft overcast light, muted grey-greens, no buildings, no roads.", seed=143, **M)
add("map_desert", C, "scene", "High aerial photograph looking straight down from far above at red rock desert: canyons, dry washes and sandstone mesas, warm low light, muted reds and ochres, no buildings, no roads.", seed=144, **M)
add("map_alpine", C, "scene", "High aerial photograph looking straight down from far above at snowy mountain ridges, grey rock and scree, cold light, muted blue-greys, no buildings, no roads.", seed=145, **M)
add("map_road", C, "scene", "High aerial photograph looking straight down from far above at a clear-cut forest: bare brown earth, rows of stumps, logging tracks and one straight unfinished gravel road, grey light, muted browns.", seed=146, **M)


def prompt(a):
    if a["kind"] == "scene":
        return a["subject"]
    if a["kind"] == "top":
        return top(a["subject"], a.get("front", "head"))
    return KINDS[a["kind"]](a["subject"])


def by_name():
    return {a["name"]: a for a in ART}


if __name__ == "__main__":
    import json

    print(json.dumps([dict(a, prompt=prompt(a)) for a in ART], indent=1))
