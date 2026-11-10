# Noughts

Noughts and crosses against the computer. You play the crosses and move
first; the computer answers each move with a nought. The game is kept by an
engine written in Clarity, with a board of its own for each open window.

## What you see and can do

The Game window holds one view, GameView:

- A canvas with the board: a grid of three by three, a cross as two lines and a nought as a ring. A click on an empty square plays it.
- A line that says how the game stands: your move, you win, I win or a draw.
- New game (Ctrl+N) empties the board.

A click on a taken square, or after the game is over, is refused with a
message. The computer completes its own line when it can,
blocks yours, takes the centre when it is free, and else picks a free square
at random.

A slide-out on the left, pinnable and resizable, shows How to play: the
rules as Markdown, from `resources/rules-en.md` or `rules-nl.md` by the
language.

The app's menu holds Game with Another window (a second board in a new
window), Preferences and Quit (Ctrl+Q). Preferences is a dialog with the
language, a setting each person keeps on their machine.

## Stack

C#, desktop. An example builds only for the stack its project file names.

## What it shows

- An engine written in Clarity: `engine Board` with `state per window`, `keep` fields for the squares and the lines, and functions the view calls (`Board.play`, `Board.mark`, `Board.restart`). It `uses random` for the computer's move.
- `refuse` with translated messages for moves that are not allowed.
- A canvas drawn from code (`ondraw`) and read on a click (`board.x`, `board.y`), in units of 300 by 300.
- A Markdown part fed from a resource per language (`gameRules resource "rules-en.md"`).
- A menu item that opens the same window again (`newWindow="true"`).
- A data draft: `history.clarity` holds Player, Game and Move, with the Mark and GameResult enums. The windows do not use them yet; it is the first draft of the ERD.

## Files

- `../noughts.clarity`: the project file (stack, output folder, the SQLite data source).
- `noughts.clarity`: the Board engine, the language setting and GameView's handlers (playing, restarting, drawing the board).
- `history.clarity`: the entities and enums of played games.
- `design/`: `Noughts.cxaml` (the app and its menu), `Game.cxaml` (the window with the help slide-out), `GameView.cxaml` (the board, status and New game), `HowToPlay.cxaml` (the rules), `Preferences.cxaml` (the dialog).
- `resources/`: the rules in English and Dutch, as Markdown.
- `translations/`: English and Dutch.
- `migrations/main/`: one step that adds Player, Game and Move.
