package com.kshana.app.net

import android.content.Context
import com.kshana.app.data.FamilyStore
import com.kshana.app.engine.EngineProvider
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.io.File
import java.io.IOException

/**
 * Refresh the IERS Earth-orientation file (finals2000A.all) the engine uses for UT1 and polar
 * motion. The bundled copy ages; a fresh one tightens the error budget for recent dates.
 * The download goes to a temp file, is sanity-checked, and only then replaces the live copy.
 */
object EopUpdate {
    const val FILE_NAME = "finals2000A.all"
    private const val MAX_BYTES = 20L * 1024 * 1024
    private const val MIN_BYTES = 1L * 1024 * 1024
    private val SOURCES = listOf(
        "https://datacenter.iers.org/data/9/finals2000A.all",
        "https://maia.usno.navy.mil/ser7/finals2000A.all",
    )

    /** Download, verify and install the file. Returns the epoch millis recorded as "fetched at". */
    suspend fun updateEop(context: Context): Result<Long> = runCatching {
        Online.require(context)
        val app = context.applicationContext
        val tmp = File(app.filesDir, "$FILE_NAME.download")
        var lastError: Exception? = null
        for (url in SOURCES) {
            try {
                Online.download(url, tmp, MAX_BYTES)
                verify(tmp)
                lastError = null
                break
            } catch (e: Exception) {
                tmp.delete()
                lastError = e
            }
        }
        if (lastError != null) throw lastError
        withContext(Dispatchers.IO) {
            val target = File(app.filesDir, FILE_NAME)
            if (!tmp.renameTo(target)) {
                // Fall back to a copy when rename is refused by the file system.
                tmp.inputStream().use { input -> target.outputStream().use { output -> input.copyTo(output) } }
                tmp.delete()
            }
        }
        EngineProvider.reset()
        val now = System.currentTimeMillis()
        FamilyStore.saveEopFetchedAt(app, now)
        now
    }

    /** Size over 1 MB and a first line in the finals format (two digits of year first). */
    private fun verify(f: File) {
        if (!f.exists() || f.length() < MIN_BYTES) throw IOException("The downloaded file is too small to be finals2000A.all")
        val first = f.bufferedReader(Charsets.US_ASCII).use { it.readLine() } ?: throw IOException("Empty download")
        if (first.length < 6 || !first[0].isDigit() || !first[1].isDigit()) {
            throw IOException("The download does not look like an IERS finals file")
        }
    }
}

suspend fun updateEop(context: Context): Result<Long> = EopUpdate.updateEop(context)
