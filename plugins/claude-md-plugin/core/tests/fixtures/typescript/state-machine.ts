/**
 * State machine example with explicit states and lifecycle.
 */

// State enum representing the possible states
export enum State {
  Idle = 'idle',
  Loading = 'loading',
  Loaded = 'loaded',
  Error = 'error',
}

export interface StateContext {
  state: State;
  data: unknown | null;
  error: Error | null;
}

/**
 * Resource loader with explicit lifecycle methods.
 */
export class ResourceLoader {
  private context: StateContext = {
    state: State.Idle,
    data: null,
    error: null,
  };

  /**
   * Initialize the loader.
   * @lifecycle 1
   */
  init(): void {
    this.context = {
      state: State.Idle,
      data: null,
      error: null,
    };
  }

  /**
   * Start loading resources.
   * @lifecycle 2
   */
  start(): void {
    if (this.context.state !== State.Idle) {
      throw new Error('Can only start from Idle state');
    }
    this.context.state = State.Loading;
    // Begin loading...
  }

  /**
   * Stop the loader.
   * @lifecycle 3
   */
  stop(): void {
    this.context.state = State.Idle;
    this.context.data = null;
  }

  /**
   * Clean up resources.
   * @lifecycle 4
   */
  destroy(): void {
    this.stop();
    // Clean up...
  }

  // State transition: Idle -> Loading
  load(): void {
    this.context.state = State.Loading;
  }

  // State transition: Loading -> Loaded
  onSuccess(data: unknown): void {
    this.context.state = State.Loaded;
    this.context.data = data;
  }

  // State transition: Loading -> Error
  onError(error: Error): void {
    this.context.state = State.Error;
    this.context.error = error;
  }

  // State transition: Error -> Idle (retry)
  retry(): void {
    if (this.context.state === State.Error) {
      this.context.state = State.Idle;
      this.context.error = null;
    }
  }
}
