package com.example.auth;

public class ExportCandidates {

    public static final int MAX_RETRIES = 3;
    public static final String API_BASE_URL = "https://api.example.com";
    public static final long DEFAULT_TIMEOUT = 30000L;

    private static final String INTERNAL_SECRET = "secret";

    public boolean processItem(String item) {
        return item != null && !item.isEmpty();
    }
}
