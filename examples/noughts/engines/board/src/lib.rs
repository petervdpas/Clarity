//! The Board engine: noughts and crosses against the computer. The engine
//! keeps the board in its memory, so the application declares it with
//! `state per window`: each open window has a game of its own. You play
//! crosses; the computer answers with a nought.

use std::sync::Mutex;

clarity_engine::interface!();

/// A square: empty, a cross (yours) or a nought (the computer's).
#[derive(Clone, Copy, PartialEq)]
enum Mark {
    Empty,
    Cross,
    Nought,
}

static BOARD: Mutex<[Mark; 9]> = Mutex::new([Mark::Empty; 9]);

/// The rows, columns and diagonals that win.
const LINES: [[usize; 3]; 8] = [[0, 1, 2], [3, 4, 5], [6, 7, 8], [0, 3, 6], [1, 4, 7], [2, 5, 8], [0, 4, 8], [2, 4, 6]];

fn winner(board: &[Mark; 9]) -> Option<Mark> {
    LINES.iter().find(|l| board[l[0]] != Mark::Empty && board[l[0]] == board[l[1]] && board[l[1]] == board[l[2]]).map(|l| board[l[0]])
}

fn full(board: &[Mark; 9]) -> bool {
    board.iter().all(|m| *m != Mark::Empty)
}

/// How the game stands, as the window shows it.
fn status(board: &[Mark; 9]) -> &'static str {
    match winner(board) {
        Some(Mark::Cross) => "You win",
        Some(_) => "I win",
        None if full(board) => "A draw",
        None => "Your move",
    }
}

/// The square that completes a line for `mark`, if there is one.
fn completing(board: &[Mark; 9], mark: Mark) -> Option<usize> {
    LINES.iter().find_map(|l| {
        let count = l.iter().filter(|&&i| board[i] == mark).count();
        let empty = l.iter().find(|&&i| board[i] == Mark::Empty);
        if count == 2 { empty.copied() } else { None }
    })
}

/// The computer's move: win, else block, else the centre, else any free square.
fn reply(board: &[Mark; 9]) -> Option<usize> {
    completing(board, Mark::Nought).or_else(|| completing(board, Mark::Cross)).or_else(|| (board[4] == Mark::Empty).then_some(4)).or_else(|| {
        let free: Vec<usize> = (0..9).filter(|&i| board[i] == Mark::Empty).collect();
        (!free.is_empty()).then(|| free[(clarity_engine::random() % free.len() as u64) as usize])
    })
}

// A new game when the window opens it.
clarity_engine::on_open!(|| {
    *BOARD.lock().map_err(|e| e.to_string())? = [Mark::Empty; 9];
    Ok(())
});

// play(square) -> text: your cross on square 1 to 9, the computer's answer,
// and how the game stands.
clarity_engine::function!(play, |(square,): (i64,)| {
    let mut board = BOARD.lock().map_err(|e| e.to_string())?;
    if winner(&board).is_some() || full(&board) {
        return Err("The game is over; start a new one".to_string());
    }
    let at = usize::try_from(square - 1).ok().filter(|&i| i < 9).ok_or("There are nine squares")?;
    if board[at] != Mark::Empty {
        return Err("That square is taken".to_string());
    }
    board[at] = Mark::Cross;
    if winner(&board).is_none()
        && let Some(answer) = reply(&board)
    {
        board[answer] = Mark::Nought;
    }
    Ok(status(&board))
});

// mark(square) -> text: what is on square 1 to 9: X, O or nothing.
clarity_engine::function!(mark, |(square,): (i64,)| {
    let board = BOARD.lock().map_err(|e| e.to_string())?;
    let at = usize::try_from(square - 1).ok().filter(|&i| i < 9).ok_or("There are nine squares")?;
    Ok(match board[at] {
        Mark::Empty => "",
        Mark::Cross => "X",
        Mark::Nought => "O",
    })
});

// restart() -> text: an empty board.
clarity_engine::function!(restart, |(): ()| {
    *BOARD.lock().map_err(|e| e.to_string())? = [Mark::Empty; 9];
    Ok("Your move")
});
