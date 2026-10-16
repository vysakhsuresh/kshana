package com.kshana.app.net

import android.content.Context
import com.kshana.app.data.FamilyStore
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.io.File
import java.io.IOException
import java.net.HttpURLConnection
import java.net.URL

/** Thrown when a network call is attempted while the user has not allowed online features. */
class OnlineDisabled : IOException("Online features are off. Turn them on in Settings to use this.")

/**
 * The one door to the network. Every optional online feature (place search, Earth-rotation
 * data, narration) goes through here, so the "online allowed" switch in Settings is honoured
 * in one place. Plain HttpURLConnection, no extra libraries, and never any logging of
 * request bodies or headers (they may carry a key).
 */
object Online {
    private const val CONNECT_TIMEOUT_MS = 10_000
    private const val READ_TIMEOUT_MS = 30_000
    private const val USER_AGENT = "Kshana/0.1 (astrology app; offline-first)"

    fun allowed(context: Context): Boolean = FamilyStore.onlineAllowed(context)

    /** Throws [OnlineDisabled] unless the user has allowed online features. */
    fun require(context: Context) {
        if (!allowed(context)) throw OnlineDisabled()
    }

    private fun connect(url: String, method: String, headers: Map<String, String>): HttpURLConnection {
        val c = URL(url).openConnection() as HttpURLConnection
        c.requestMethod = method
        c.connectTimeout = CONNECT_TIMEOUT_MS
        c.readTimeout = READ_TIMEOUT_MS
        c.instanceFollowRedirects = true
        c.setRequestProperty("User-Agent", USER_AGENT)
        for ((k, v) in headers) c.setRequestProperty(k, v)
        return c
    }

    private fun readBody(c: HttpURLConnection): String {
        val stream = if (c.responseCode in 200..299) c.inputStream else (c.errorStream ?: c.inputStream)
        return stream.bufferedReader(Charsets.UTF_8).use { it.readText() }
    }

    /** GET a text resource. Throws IOException with the status code on a non-2xx response. */
    suspend fun getText(url: String, headers: Map<String, String> = emptyMap()): String = withContext(Dispatchers.IO) {
        val c = connect(url, "GET", headers)
        try {
            val body = readBody(c)
            if (c.responseCode !in 200..299) throw IOException("HTTP ${c.responseCode} from ${URL(url).host}")
            body
        } finally {
            c.disconnect()
        }
    }

    /**
     * POST a JSON body and return the response body as text, whatever the status. Callers that
     * need the status code use [postJsonWithStatus]; this one throws on non-2xx.
     */
    suspend fun postJson(url: String, body: String, headers: Map<String, String> = emptyMap()): String {
        val (code, text) = postJsonWithStatus(url, body, headers)
        if (code !in 200..299) throw IOException("HTTP $code from ${URL(url).host}: ${text.take(300)}")
        return text
    }

    /** POST a JSON body; returns (status code, response body) without throwing on 4xx/5xx. */
    suspend fun postJsonWithStatus(url: String, body: String, headers: Map<String, String> = emptyMap()): Pair<Int, String> =
        withContext(Dispatchers.IO) {
            val c = connect(url, "POST", headers)
            c.doOutput = true
            if (headers.keys.none { it.equals("content-type", ignoreCase = true) }) {
                c.setRequestProperty("Content-Type", "application/json")
            }
            try {
                c.outputStream.use { it.write(body.toByteArray(Charsets.UTF_8)) }
                Pair(c.responseCode, readBody(c))
            } finally {
                c.disconnect()
            }
        }

    /**
     * Download a resource to a file, stopping with an IOException if it exceeds [maxBytes].
     * The file is written fully or deleted.
     */
    suspend fun download(url: String, to: File, maxBytes: Long) = withContext(Dispatchers.IO) {
        val c = connect(url, "GET", emptyMap())
        try {
            if (c.responseCode !in 200..299) throw IOException("HTTP ${c.responseCode} from ${URL(url).host}")
            var total = 0L
            val buf = ByteArray(64 * 1024)
            try {
                c.inputStream.use { input ->
                    to.outputStream().use { output ->
                        while (true) {
                            val n = input.read(buf)
                            if (n < 0) break
                            total += n
                            if (total > maxBytes) throw IOException("Download exceeded $maxBytes bytes")
                            output.write(buf, 0, n)
                        }
                    }
                }
            } catch (e: Exception) {
                to.delete()
                throw e
            }
        } finally {
            c.disconnect()
        }
    }
}
