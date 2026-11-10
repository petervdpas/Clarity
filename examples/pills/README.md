# Pills

Break the wall of pills with a bouncing ball. You keep the ball up with a
paddle; each pill it hits breaks and scores a point. A game has three balls.
The game runs in an engine written in Clarity beside the screen.

## What you see and can do

The Play window holds one view, PlayView:

- The field: four rows of eight pills (red, yellow, green, blue), the paddle and the ball.
- The paddle follows the mouse, or the left and right arrow keys while they are down.
- Where the ball lands on the paddle sets the angle it leaves at.
- Score and Balls: two digit displays beside the field.
- Start begins a new game with a full wall and three balls.

A ball that falls below the paddle costs one ball. When the wall is gone a
new one is built. After the last ball a message gives the score.

## Stack

Go, desktop. An example builds only for the stack its project file names.

## What it shows

- An engine written in Clarity that runs beside the screen: `engine Breaker` keeps the game's state, steps on `clarity_tick`, takes `clarity_pointer` and `clarity_key`, and returns its shapes from `clarity_draw`.
- `engines interpreted` in the project file: Clarity's interpreter runs the engine.
- A canvas that runs an engine (`runs="Breaker"`), with the engine's tells (`hit`, `lost`, `over`) wired to the view's handlers and `field.start()` sending it `start`.
- Pictures from the resources folder as constants (`const pillRed: picture = "pill-red.svg"`), drawn as images.

## Files

- `../pills.clarity`: the project file (stack, output folder, `engines interpreted`, the SQLite data source).
- `pills.clarity`: the picture constants, the Shape record, the Breaker engine and PlayView's handlers.
- `design/`: `Pills.cxaml` (the app), `Play.cxaml` (the window), `PlayView.cxaml` (the field, displays and Start).
- `resources/`: the four pill pictures and the paddle, as SVG.
- `translations/`: English and Dutch.
