package com.kshana.app.engine

import android.content.Context
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.Engine
import java.io.File
import uniffi.kshana_ffi.setRiseConvention
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

    /**
     * The bundled planet file: JPL DE440s (1849-12-26 … 2150-01-22), so births back to 1850
     * and forecasts to 2150 can be read. Builds that predate it shipped DE421 (1900–2053); a
     * copy left in the files directory by such a build is removed once DE440s is in place.
     */
    private const val KERNEL = "de440s.bsp"
    private const val OLD_KERNEL = "de421.bsp"

    private fun open(context: Context): Engine {
        val spk = runCatching { ensureAsset(context, KERNEL) }
            .onSuccess { runCatching { File(context.filesDir, OLD_KERNEL).delete() } }
            .getOrElse { ensureAsset(context, OLD_KERNEL) }
        val eop = runCatching { ensureAsset(context, "finals2000A.all") }.getOrNull()
        runCatching { setRiseConvention(ProfileStore.settings(context).riseConvention) }
        return Engine.open(spk.absolutePath, eop?.absolutePath)
    }

    private fun ensureAsset(context: Context, name: String): File {
        val target = File(context.filesDir, name)
        if (!target.exists() || target.length() == 0L) {
            // Copy to a temporary name first so a killed process never leaves half a kernel.
            val part = File(context.filesDir, "$name.part")
            context.assets.open(name).use { input ->
                part.outputStream().use { output -> input.copyTo(output) }
            }
            if (!part.renameTo(target)) {
                part.delete()
                error("could not install $name")
            }
        }
        return target
    }
}
