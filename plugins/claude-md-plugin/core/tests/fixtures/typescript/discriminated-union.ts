// Discriminated union state pattern
type State =
    | { kind: 'idle' }
    | { kind: 'loading'; progress: number }
    | { kind: 'loaded'; data: string }
    | { kind: 'error'; message: string };

// Alternative with 'type' discriminator
type Event =
    | { type: 'START' }
    | { type: 'DATA_RECEIVED'; payload: string }
    | { type: 'CANCEL' };

// Interface-based discriminated union
interface IdleState { status: 'idle' }
interface LoadingState { status: 'loading'; progress: number }
interface LoadedState { status: 'loaded'; data: string }
interface ErrorState { status: 'error'; message: string }

type AppState = IdleState | LoadingState | LoadedState | ErrorState;
