# Data events

How to run code whenever a record is saved or deleted, from any window or action, and how a refusal stops it.

## An entity's code block

Data events live in a `code` block with the entity's name, in the same file as the entity. They run whenever one of its records is saved or deleted: from a handler, an action, a table edit or another data event.

```clarity
code Order
    before save
        if order.status == Approved and not within_credit(order)
            refuse @orderOverCreditLimit(number = order.number)

    after save
        log_change("saved " + order.number)

    before delete
        if order.status != Pending
            refuse @onlyPendingDeletable
```

| Event | Runs |
|---|---|
| `before save` | before a record is stored; what it changes in the record is stored too |
| `after save` | after it is stored; a new record has its id by then |
| `before delete` | before a record is removed |
| `after delete` | after it is removed |

Each event is written once per entity. An entity's code block holds only these events, no handlers.

## The record

The record is named after the entity in snake case: `order` in the events of `Order`, `order_line` in those of `OrderLine`. A new record has id 0 in `before save`, so `order.id == 0` tells you it is new.

## What an event can do

An event works like an action on its record. It can read and change records, `save` and `delete` others, call functions and actions, and reach files. It declares no effects.

It does not talk to the screen: `message`, `open`, `close` and part properties are not allowed. To tell the person something, refuse.

An event that saves its own record in a save event, or deletes it in a delete event, would run itself again without end. The checker reports it.

## Refusing

`refuse "text"` stops the save or delete. The text is the error the window shows, so write it for the person using the application. It can hold slots: `refuse "{loan.album.title} is with another friend already"`.

## One transaction

A save or delete runs in one transaction with its events, and with the saves and deletes those events make in the same database. When an event fails or refuses, none of it is kept, and a new record is still new.

Saves into another database are not part of that transaction yet. Two saves in one handler are two transactions: the first stays when the second is refused.

## Where to write them

Open a module from Modules in the project tree. Under **Data events** each entity of the module is listed. **Add events** writes the entity's code block with a first `before save` and opens it as text; an entity that has events shows which ones, and opens them when you pick it. The Code panel of the window designer does not show data events.

Renaming an entity renames its code block and the record's name in its events.

See also [Functions and actions](code.functions.md) and [Entities](data.entities.md). The full rules are in [the language reference](../LANGUAGE.md#data-events).
