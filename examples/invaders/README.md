# Invaders

A space invaders game. A fleet of forty marches from side to side and down
while you shoot it from your ship. The game's engine is written in Clarity
and runs beside the screen. The program is a web server, and the game plays
in the browser that visits it.

## What you see and can do

The Play window holds one view, PlayView:

- The playing field, a canvas of 160 by 120 units: the fleet in five rows of eight, your ship at the bottom, your shot and the fleet's bombs.
- Score, Lives and Wave on the right, and **Start**, which begins a new game with three lives.
- A line under the field says how to play: move with the mouse or the arrow keys, click or press space to shoot.

You have one shot at a time. An invader in the top row is worth 30 points,
the next two rows 20, the bottom two 10. The fleet marches faster as it
thins and as the waves go on, and one invader at random drops a bomb every
0.7 seconds. A bomb on your ship costs a life. Clearing the fleet starts the
next wave. The game ends when you have no lives left or the fleet reaches
the bottom, and a message shows your score.

Each invader has two pictures, one per step of its march: a squid, a crab
and an octopus. The ship has a picture of its own.

## Stack

Go, web. An example builds only for the stack its project file names.

## What it shows

- An engine written in Clarity: `engine Fleet` runs `beside the screen` and keeps its own state (`keep`). It gets time (`clarity_tick`), the pointer (`clarity_pointer`) and the keys (`clarity_key`), and returns the shapes to draw (`clarity_draw`).
- Talk between the engine and the view: the engine `tells` hit, lost, cleared and over, which the canvas passes to the view's handlers; the view calls `field.start()`, which the engine `hears`.
- `uses random` for the bombs.
- Pictures from `resources/`, named as `picture` constants and drawn by the canvas.

## Files

- `../invaders.clarity`: the project file (title, stack, language, the SQLite data source).
- `invaders.clarity`: the picture constants, the Shape record, the Fleet engine and PlayView's handlers.
- `design/`: `Invaders.cxaml` (the app), `Play.cxaml` (the window), `PlayView.cxaml` (the field, the score panel and Start).
- `resources/`: the SVG pictures: `squid-1`, `squid-2`, `crab-1`, `crab-2`, `octopus-1`, `octopus-2` and `ship`.
- `translations/`: English and Dutch.
