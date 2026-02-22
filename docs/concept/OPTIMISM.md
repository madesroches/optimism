# OPTIMISM

A Pacman-inspired game loosely based on Voltaire's *Candide, ou l'Optimisme*.

Game over screen: **"The best of possible games. Or was it?"**

---

## Player

**Candide** — naive, wide-eyed, always moving forward. His sprite evolves throughout the game as he collects items and picks up weapons. By late game he can look completely ridiculous.

## Narrator

**Pangloss** — ever-present voice. Delivers absurd optimistic commentary as everything goes wrong. Grows obviously insane as the violence escalates.

---

## Core Mechanics

### Money (Dots)

Collect all the money in the maze to clear the level. That's the whole objective. Simple, honest, Voltairean.

### Weapons (Power Pellets)

Close-range only. Every kill is a commitment — you have to chase them down and get right next to them. Weapons escalate across levels:

| Levels | Weapon | Notes |
|---|---|---|
| 1-2 | **Brass knuckles** | Crude, ugly, up close. One hit, they drop. Sets the tone immediately — no pretense about what kind of game this is. |
| 3-4 | **Bat** | Solid. Satisfying thwack. Candide stands straighter. |
| 5-6 | **Knife** | Quick, quiet. Animations get shorter. Less struggle. |
| 7-8 | **Axe** | Heavy. Screen shake on contact. Enemies take a second to drop. |
| 9+ | **Chainsaw** | Loud, messy, completely disproportionate. The rev. The buzz. Over harpsichord music and enlightenment quotes. Enemies scatter in terror — total inversion from level 1. |

### Luxury Items (Fruit Bonuses)

Appear twice per level in a dangerous central spot. Temporary — grab them or they vanish. Pure greed trap. Kétaine as hell.

Each item **visually changes Candide's sprite** when collected.

Collecting a luxury item makes **the Thief faster** for the rest of the level. More wealth on you, more attention.

| Levels | Item | Visual Effect |
|---|---|---|
| 1-2 | **Gold grill** | Candide's pixel mouth goes shiny |
| 3-4 | **Big chain necklace** | Comically oversized, drags on the floor behind him |
| 5-6 | **Rolex** | Gleams on his wrist |
| 7-8 | **Jewel-encrusted goblet** | Carries it above his head like a trophy |
| 9-10 | **Fur coat** | Big fluffy coat drawn within the same tile frame. Absurdly puffy. |
| 11+ | **Gold toilet** | Peak kétaine. No justification. It just sits there glowing and you risk your life for it. |

---

## Enemies — The Misfortunes

Not abstract concepts. *People.* Suffering isn't cosmic, it's inflicted by other people.

| Enemy | Color | Behavior | Kill Animation (on Candide) |
|---|---|---|---|
| **The Soldier** | Red | Direct pursuit, fastest | Runs you through with a bayonet. Quick, violent. |
| **The Inquisitor** | Purple | Methodical, cuts off exits | Auto-da-fé. You burn. Dramatic, flames. |
| **The Thief** | Yellow | Erratic, steals money on contact before killing | Robs you first (money counter drops), then a knife. Double punishment. |
| **The Brute** | Green | Slower but persistent | Crushes you. Heavy, brutal. |

### Enemy Deaths

When Candide kills an enemy with a weapon:
- Brief animation — the weapon connects. They drop. A small pool of pixels.
- After a beat, they drag themselves back to the center and regenerate.
- Because these are *types*, not individuals. You killed a soldier but The Soldier returns.

---

## Levels — Candide's Journey

*(To be designed)*

---

## The Garden (Final Level)

After the chainsaw levels — silence. A small, simple maze. No enemies. No weapons. No Pangloss. Just money to collect quietly. You tend the garden.

All the gaudy luxury items are gone. The contrast does all the work. This is the entire point of *Candide* expressed in two levels.

---

## Pangloss Quotes

Samples:

- On collecting money: *"Wealth surely finds its way to those most deserving in this, the best of all possible economies"*
- On weapon pickup: *"A necessary instrument of justice in the best of all possible worlds"*
- On player death: *"A necessary evil in the best of all possible worlds"*
- On the luxury gold toilet: *"Surely the finest of all possible commodes"*
- On killing an enemy: *"He surely deserved it in this, the most just of all possible worlds"*
- As things get worse: *"This is surely the best of all possible mazes"*

---

## Tone & Style

- **Visuals**: Colorful, whimsical pixel art. Cheerful on the surface.
- **Music**: Upbeat harpsichord that slowly gets more frantic. Briefly swells into church organ during power-ups. Chainsaw levels: the buzzing drowns the music.
- **Humor**: Dry. The game never acknowledges the absurdity directly — it just commits to the bit.
- **Game over**: Stats screen — misfortunes endured, money collected, luxury items grabbed, kills by weapon type. Titled: *"The best of possible games... considering..."*
