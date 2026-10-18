package com.kshana.app

import android.app.Application
import com.kshana.app.system.CrashLog

/** The application: installs the on-phone crash log before anything else runs. */
class KshanaApp : Application() {
    override fun onCreate() {
        super.onCreate()
        CrashLog.install(this)
    }
}
