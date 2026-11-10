# Ledger

A command-line program that keeps the money of your accounts. You add
accounts, book income and expenses on them, and list the entries and the
balances. The accounts and entries are kept in a SQLite database. The
program has no windows.

## What you can do

The program is called `ledger`; `ledger --help` lists its commands.

- `account add <name>` adds an account, with the option `opened` for the day it opened (today when left out). A name can be used once.
- `account list` lists the accounts with the day each opened.
- `account remove <name>` asks first, then removes the account and everything booked on it.
- `account use <name>` sets the account `book` books on when it names none. This is a setting, kept for each person on their machine.
- `book <amount>` books an amount, as income unless the option `expense` is given. Further options: `account`, `day` (today when left out) and `note`. The amount must be more than 0.
- `entries <name>` lists what was booked on an account (booked, side, amount, note) and ends with its balance.
- `report` lists every account with its balance. When one is below zero, it warns and ends with exit code 3, so a script can check it.

Amounts are money in euros (`currency EUR` in the project file).

## Stack

C#, CLI. An example builds only for the stack its project file names.

## What it shows

- A command-line app: `cli Ledger` and a `command` per verb, with arguments, options and help texts from the translations.
- Questions on the command line (`ask ... yes @remove no @keep`), refusals and an exit code.
- An `action` with declared effects (`effects db.read`) that works out a balance, shared by `entries` and `report`.
- A setting changed from a command (`settings.account`).

## Files

- `../ledger.clarity`: the project file (title, stack, language, currency, the SQLite data source).
- `ledger.clarity`: the Side enum, the Account and Entry entities, the settings, the `balance` action and the commands.
- `translations/`: English and Dutch.
- `migrations/main/`: the step that adds the Account and Entry tables.
