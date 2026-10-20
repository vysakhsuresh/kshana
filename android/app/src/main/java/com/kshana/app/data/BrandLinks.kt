package com.kshana.app.data

import android.content.ActivityNotFoundException
import android.content.Context
import android.content.Intent
import android.net.Uri

/**
 * Company-wide links, the same destinations Sheaf, Deja, Abhyas and LayerLink use. Kept in
 * step by hand rather than shared. Opening one hands the address to another app (mail,
 * WhatsApp, the browser); nothing about the person's readings travels with it.
 */
object BrandLinks {
    const val WEBSITE_URL = "https://layerbit.co.in"
    /** How the company is named in the UI. */
    const val BRAND_LABEL = "Layerbit AI"
    const val WHATSAPP_URL = "https://wa.me/916282595823"
    const val SUPPORT_EMAIL = "ceo@layerbit.co.in"

    fun openUrl(context: Context, url: String) {
        try {
            context.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(url)).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
        } catch (_: ActivityNotFoundException) {
            // No browser, or WhatsApp not installed: nothing sensible to fall back to.
        }
    }

    fun sendEmail(context: Context, subject: String, body: String = "") {
        val intent = Intent(Intent.ACTION_SENDTO).apply {
            data = Uri.parse("mailto:$SUPPORT_EMAIL")
            putExtra(Intent.EXTRA_SUBJECT, subject)
            putExtra(Intent.EXTRA_TEXT, body)
        }
        try {
            context.startActivity(intent)
        } catch (_: ActivityNotFoundException) {
            openUrl(context, WEBSITE_URL)
        }
    }
}
