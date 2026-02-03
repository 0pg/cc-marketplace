package com.example.state

/**
 * State machine example for Kotlin code analyzer.
 */
class StateMachine {

    /**
     * State enum representing possible states.
     */
    enum class State {
        IDLE,
        LOADING,
        LOADED,
        ERROR
    }

    private var state: State = State.IDLE
    private var data: Any? = null
    private var error: Exception? = null

    /**
     * Initialize the state machine.
     * @lifecycle 1
     */
    fun init() {
        this.state = State.IDLE
        this.data = null
        this.error = null
    }

    /**
     * Start the state machine.
     * @lifecycle 2
     */
    @Throws(IllegalStateException::class)
    fun start() {
        if (this.state != State.IDLE) {
            throw IllegalStateException("Can only start from IDLE state")
        }
        this.state = State.LOADING
    }

    /**
     * Stop the state machine.
     * @lifecycle 3
     */
    fun stop() {
        this.state = State.IDLE
        this.data = null
    }

    /**
     * Destroy the state machine.
     * @lifecycle 4
     */
    fun destroy() {
        stop()
    }

    /**
     * State transition: IDLE -> LOADING
     */
    fun load() {
        this.state = State.LOADING
    }

    /**
     * State transition: LOADING -> LOADED
     */
    fun onSuccess(data: Any) {
        this.state = State.LOADED
        this.data = data
    }

    /**
     * State transition: LOADING -> ERROR
     */
    fun onError(error: Exception) {
        this.state = State.ERROR
        this.error = error
    }

    /**
     * State transition: ERROR -> IDLE (retry)
     */
    fun retry() {
        if (this.state == State.ERROR) {
            this.state = State.IDLE
            this.error = null
        }
    }

    /**
     * Get current state.
     */
    fun getState(): State = state
}
