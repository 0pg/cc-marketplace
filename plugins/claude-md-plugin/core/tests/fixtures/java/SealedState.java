package com.example.state;

public sealed class State permits Idle, Loading, Loaded, Error {
}

final class Idle extends State {
}

final class Loading extends State {
    private final int progress;
    public Loading(int progress) { this.progress = progress; }
}

final class Loaded extends State {
    private final String data;
    public Loaded(String data) { this.data = data; }
}

final class Error extends State {
    private final String message;
    public Error(String message) { this.message = message; }
}
