package com.kshana.app.ui.sky

import android.content.Context
import androidx.compose.runtime.Composable
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.Font
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight

/*
 * The night typography, bundled under res/font and resolved by name:
 *
 *   serif  Cormorant Garamond, instanced at wght 500 and subset to Latin (cormorantgaramond_medium.ttf)
 *   sans   Manrope variable, one file for weights 200–800 (manrope_wght.ttf)
 *   mono   IBM Plex Mono Regular and Medium (ibmplexmono_regular.ttf, ibmplexmono_medium.ttf)
 *
 * The resources are looked up with `Resources.getIdentifier` rather than `R.font.*`, so the
 * families degrade to the system Serif / SansSerif / Monospace when a file is missing (the
 * type-check script has no resource compiler) and so a broken font never crashes a screen:
 * every lookup is inside `runCatching`. Astrological glyphs (☉ ☽ ♈ …) are not in these
 * families and fall through to the system symbol fonts as before.
 */
object KshanaFonts {
    private class Families(val serif: FontFamily, val sans: FontFamily, val mono: FontFamily)

    @Volatile private var families: Families? = null
    @Volatile private var attempted = false

    /** Cormorant Garamond Medium when bundled, else the system serif. */
    val serif: FontFamily get() = families?.serif ?: FontFamily.Serif

    /** Manrope when bundled, else the system sans. */
    val sans: FontFamily get() = families?.sans ?: FontFamily.SansSerif

    /** IBM Plex Mono when bundled, else the system monospace. */
    val mono: FontFamily get() = families?.mono ?: FontFamily.Monospace

    /** True once at least one bundled family resolved. */
    val bundled: Boolean get() = families != null

    /**
     * Resolve the bundled files once per process. Safe to call from any composable; the
     * living sky calls it so every screen that shows the sky has the fonts before its text.
     */
    fun attach(context: Context) {
        if (attempted) return
        synchronized(this) {
            if (attempted) return
            attempted = true
            families = runCatching { load(context.applicationContext) }.getOrNull()
        }
    }

    private fun fontId(context: Context, name: String): Int =
        runCatching { context.resources.getIdentifier(name, "font", context.packageName) }.getOrDefault(0)

    private fun load(context: Context): Families? {
        val serifId = fontId(context, "cormorantgaramond_medium")
        val sansId = fontId(context, "manrope_wght")
        val monoRegularId = fontId(context, "ibmplexmono_regular")
        val monoMediumId = fontId(context, "ibmplexmono_medium")
        if (serifId == 0 && sansId == 0 && monoRegularId == 0) return null

        // One static instance stands in for every serif weight the theme asks for.
        val serif = if (serifId != 0) {
            FontFamily(
                Font(serifId, FontWeight.Normal),
                Font(serifId, FontWeight.Medium),
                Font(serifId, FontWeight.SemiBold),
            )
        } else FontFamily.Serif

        // Manrope is variable: on API 26+ Compose sets 'wght' from the requested weight.
        val sans = if (sansId != 0) {
            FontFamily(
                Font(sansId, FontWeight.Light),
                Font(sansId, FontWeight.Normal),
                Font(sansId, FontWeight.Medium),
                Font(sansId, FontWeight.SemiBold),
                Font(sansId, FontWeight.Bold),
            )
        } else FontFamily.SansSerif

        val mono = when {
            monoRegularId != 0 && monoMediumId != 0 -> FontFamily(
                Font(monoRegularId, FontWeight.Normal),
                Font(monoMediumId, FontWeight.Medium),
            )
            monoRegularId != 0 -> FontFamily(Font(monoRegularId, FontWeight.Normal))
            else -> FontFamily.Monospace
        }
        return Families(serif, sans, mono)
    }
}

/** Attach the bundled fonts to this composition's context and return the families. */
@Composable
fun rememberKshanaFonts(): KshanaFonts {
    KshanaFonts.attach(LocalContext.current)
    return KshanaFonts
}
