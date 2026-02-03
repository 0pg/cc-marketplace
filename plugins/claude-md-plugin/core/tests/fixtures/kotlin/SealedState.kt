package com.example.state

sealed class State {
    object Idle : State()
    data class Loading(val progress: Int) : State()
    data class Loaded(val data: String) : State()
    data class Error(val message: String) : State()
}

sealed interface Event {
    object Start : Event
    data class DataReceived(val data: String) : Event
    object Cancel : Event
}
