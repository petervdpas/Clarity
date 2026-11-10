# Kitchen

A recipe book. Each recipe has an overview with a photo, its ingredients
and its method, and a kitchen timer beeps when the time is up. The recipes
are kept in a SQLite database.

## What you see and can do

The Cookbook window holds one view, CookbookView. On the left is a table of
the recipes (name, course, minutes), sortable, with add and remove. The
recipe picked opens on the right in a horizontal accordion of four sections:

- Overview: the recipe's name, course (starter, main or dessert), how many it serves, how many minutes it takes, notes and a photo. **Save** stores it and the list on the left shows it.
- Ingredients: a table of name and amount, edited in place. Add, remove and drag to reorder.
- Method: the steps in order, dragged to move. Picking a step shows its whole text in a box below, and **Save** stores it.
- Timer: minutes to count down (it takes the recipe's minutes when you pick one), **Start** and **Stop**, and the seconds left.

Adding a recipe opens an empty overview. Its ingredients and method can be
written once it is saved. Removing a recipe asks first, and removes its
ingredients and steps with it. When the timer reaches zero it plays twenty
beeps and a message says the time is up. The footer reads "Pick a recipe,
or add one".

## Stack

C, desktop. An example builds only for the stack its project file names.

## What it shows

- An accordion whose sections are views, each in a content of its own, and a last section that holds parts directly.
- A view that tells its window: RecipeOverview declares `<Tells message="saved" />`, and the window handles it with `onsaved="recipeSaved"`.
- Tables with in-place editing, confirmation on remove and drag to reorder (`reorder="position"`).
- An image field on an entity, shown in an Image part.
- A Timer part ticking every second, and a Sound part playing a melody.

## Files

- `../kitchen.clarity`: the project file (title, stack, language, the SQLite data source).
- `kitchen.clarity`: the Course enum, the Recipe, Ingredient and Step entities, and the code behind the window and each view.
- `design/`: `Kitchen.cxaml` (the app), `Cookbook.cxaml` (the window and its footer), `CookbookView.cxaml` (the recipe list, the accordion, the timer and sound), `RecipeOverview.cxaml`, `IngredientList.cxaml` and `MethodSteps.cxaml` (the three views in the accordion).
- `translations/`: English and Dutch.
- `migrations/main/`: two steps: the recipes with their ingredients and steps, then the photo.
