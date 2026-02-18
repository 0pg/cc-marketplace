package com.example.auth

const val MAX_RETRIES = 3
val DEFAULT_TIMEOUT: Long = 30000L
val API_BASE_URL = "https://api.example.com"

typealias UserId = String
typealias TokenMap = Map<String, String>

object AppConfig {
    val version = "1.0.0"
}

object DatabaseManager {
    fun connect() {}
}

interface Validator {
    fun validate(): Boolean
}

interface Serializable {
    fun serialize(): ByteArray
}

fun processItem(item: String): Boolean {
    return item.isNotEmpty()
}
