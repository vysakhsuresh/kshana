package com.kshana.app.ui.screens

import uniffi.kshana_ffi.ChartRecord
import uniffi.kshana_ffi.lunarNode

/**
 * Rahu's tropical longitude by the engine-wide node choice (Settings → Rahu and Ketu):
 * the true node by default, the mean node if chosen. The engine uses the same choice for
 * the natal chart, transits and timings, so every screen shows one Rahu.
 */
internal val ChartRecord.rahuTropical: Double
    get() = if (runCatching { lunarNode() }.getOrNull() == "mean") meanNodeTropical else trueNodeTropical
