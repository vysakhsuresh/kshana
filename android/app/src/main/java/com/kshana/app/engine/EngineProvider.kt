package com.kshana.app.engine

import android.content.Context
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.Engine
import java.io.File
import uniffi.kshana_ffi.setRiseConvention
import uniffi.kshana_ffi.setLunarNode
import com.kshana.app.data.ProfileStore

/**
 * Opens the Rust engine once per process. The kernel ships in the APK's assets and is
 * copied to the app's files directory on first launch, because the Rust reader needs a
 * real file path. Nothing is downloaded.
 */
object EngineProvider {
    @Volatile private var engine: Engine? = null

    suspend fun get(context: Context): Engine = engine ?: withContext(Dispatchers.IO) {
        engine ?: open(context.applicationContext).also { engine = it }
    }

    /**
     * Forget the open engine so the next [get] reopens it, e.g. after a fresh Earth-orientation
     * file has been written to the files directory. The old handle stays valid for callers that
     * already hold it.
     */
    fun reset() {
        engine = null
    }

    private fun open(context: Context): Engine {
        val spk = ensureAsset(context, "de421.bsp")
        val eop = runCatching { ensureAsset(context, "finals2000A.all") }.getOrNull()
        runCatching { setRiseConvention(ProfileStore.settings(context).riseConvention) }
        runCatching { setLunarNode(ProfileStore.settings(context).lunarNode) }
        return Engine.open(spk.absolutePath, eop?.absolutePath)
    }

    private fun ensureAsset(context: Context, name: String): File {
        val target = File(context.filesDir, name)
        if (!target.exists() || target.length() == 0L) {
            context.assets.open(name).use { input ->
                target.outputStream().use { output -> input.copyTo(output) }
            }
        }
        return target
    }
}
