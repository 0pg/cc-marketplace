package com.example.state;

/**
 * State machine example for Java code analyzer.
 */
public class StateMachine {

    /**
     * State enum representing possible states.
     */
    public enum State {
        IDLE,
        LOADING,
        LOADED,
        ERROR
    }

    private State state = State.IDLE;
    private Object data = null;
    private Exception error = null;

    /**
     * Initialize the state machine.
     * @lifecycle 1
     */
    public void init() {
        this.state = State.IDLE;
        this.data = null;
        this.error = null;
    }

    /**
     * Start the state machine.
     * @lifecycle 2
     */
    public void start() throws IllegalStateException {
        if (this.state != State.IDLE) {
            throw new IllegalStateException("Can only start from IDLE state");
        }
        this.state = State.LOADING;
    }

    /**
     * Stop the state machine.
     * @lifecycle 3
     */
    public void stop() {
        this.state = State.IDLE;
        this.data = null;
    }

    /**
     * Destroy the state machine.
     * @lifecycle 4
     */
    public void destroy() {
        stop();
    }

    /**
     * State transition: IDLE -> LOADING
     */
    public void load() {
        this.state = State.LOADING;
    }

    /**
     * State transition: LOADING -> LOADED
     */
    public void onSuccess(Object data) {
        this.state = State.LOADED;
        this.data = data;
    }

    /**
     * State transition: LOADING -> ERROR
     */
    public void onError(Exception error) {
        this.state = State.ERROR;
        this.error = error;
    }

    /**
     * State transition: ERROR -> IDLE (retry)
     */
    public void retry() {
        if (this.state == State.ERROR) {
            this.state = State.IDLE;
            this.error = null;
        }
    }

    /**
     * Get current state.
     */
    public State getState() {
        return state;
    }
}
