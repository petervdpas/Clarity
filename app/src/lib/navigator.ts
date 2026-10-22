// How windows open and close in a running application, on every stack alike
// (DESIGN.md, Page kinds; cases in conformance/windows.json): `open` shows a
// window in place on top of the one that opened it, `close` goes back to the
// one below, which is told (`on return`). A Navigator keeps the windows of
// one desktop window; a target marked `in new window` opens another desktop
// window with a Navigator of its own. Closing the last window of a desktop
// window ends the application, or for another desktop window closes it and
// tells its opener. A handler that closes its window and opens others closes
// first.

/** What the backend answers when a window opens, closes, returns or runs a handler. */
export type NavReply<S, M, A = never> = {
  state: S;
  messages: M[];
  open: { window: string; with: Record<string, unknown>; newWindow?: boolean }[];
  close: boolean;
  /** The questions its handler asked (DESIGN 5.14), asked when it has finished. */
  asks?: A[];
};

/** The backend calls that open and close windows. */
export type NavBackend<S, M, A = never> = {
  /** Loads a window's data and runs its `on open`. */
  open: (window: string, withData: Record<string, unknown>) => Promise<NavReply<S, M, A>>;
  /** Runs a window's `on close`. */
  close: (state: S) => Promise<NavReply<S, M, A>>;
  /** Runs `on return` on the window that is on top again. */
  returnTo: (state: S) => Promise<NavReply<S, M, A>>;
  /** Opens a window in a desktop window of its own, as the design marks it. */
  openApart: (window: string, withData: Record<string, unknown>) => Promise<void>;
  /** The last window has closed: ends the application, or closes this desktop window and tells its opener. */
  quit: () => Promise<void>;
};

/** The stack of open windows, bottom first, kept where the renderer shows it. */
export type NavStack<S> = { get: () => S[]; set: (stack: S[]) => void };

export class Navigator<S, M, A = never> {
  private readonly stack: NavStack<S>;
  private readonly backend: NavBackend<S, M, A>;
  private readonly say: (message: M) => void;
  private readonly fail: (err: unknown) => void;
  private readonly ask: (question: A, window: number) => void;

  /** `ask` gets each question a reply asks, with the place in the stack of the window that asked. */
  constructor(stack: NavStack<S>, backend: NavBackend<S, M, A>, say: (message: M) => void, fail: (err: unknown) => void, ask: (question: A, window: number) => void = () => {}) {
    this.stack = stack;
    this.backend = backend;
    this.say = say;
    this.fail = fail;
    this.ask = ask;
  }

  /** Opens a window on top. */
  async open(window: string, withData: Record<string, unknown> = {}): Promise<void> {
    try {
      const reply = await this.backend.open(window, withData);
      this.stack.set([...this.stack.get(), reply.state]);
      await this.follow(reply, this.stack.get().length - 1);
    } catch (err) {
      this.fail(err);
    }
  }

  /** Closes the top window after its `on close`, then runs `on return` on the one below. */
  async close(): Promise<void> {
    const top = this.stack.get().at(-1);
    if (top === undefined) return;
    try {
      const closing = await this.backend.close(top);
      closing.messages.forEach(this.say);
      const rest = this.stack.get().slice(0, -1);
      this.stack.set(rest);
      const below = rest.at(-1);
      if (below === undefined) {
        await this.backend.quit();
        return;
      }
      const back = await this.backend.returnTo(below);
      this.replace(rest.length - 1, back.state);
      await this.follow(back, rest.length - 1);
    } catch (err) {
      this.fail(err);
    }
  }

  /** A window this desktop window opened elsewhere has closed: the window on top is told. */
  async returned(): Promise<void> {
    const top = this.stack.get().at(-1);
    if (top === undefined) return;
    try {
      const back = await this.backend.returnTo(top);
      this.replace(this.stack.get().length - 1, back.state);
      await this.follow(back, this.stack.get().length - 1);
    } catch (err) {
      this.fail(err);
    }
  }

  /** Opens a window as a click target says: in place, or in a new desktop window. */
  async target(window: string, newWindow: boolean): Promise<void> {
    if (newWindow) await this.openApart(window, {});
    else await this.open(window);
  }

  private async openApart(window: string, withData: Record<string, unknown>): Promise<void> {
    try {
      await this.backend.openApart(window, withData);
    } catch (err) {
      this.fail(err);
    }
  }

  /** Applies a handler's reply to the window at `index`. */
  async apply(index: number, reply: NavReply<S, M, A>): Promise<void> {
    this.replace(index, reply.state);
    await this.follow(reply, index);
  }

  /**
   * What a reply of the window at `window` asks for: its messages, closing
   * its window first, then the windows it opens, then its questions.
   */
  async follow(reply: NavReply<S, M, A>, window: number): Promise<void> {
    reply.messages.forEach(this.say);
    if (reply.close) await this.close();
    for (const request of reply.open) {
      if (request.newWindow) await this.openApart(request.window, request.with);
      else await this.open(request.window, request.with);
    }
    for (const question of reply.asks ?? []) this.ask(question, window);
  }

  private replace(index: number, state: S) {
    const next = [...this.stack.get()];
    next[index] = state;
    this.stack.set(next);
  }
}
