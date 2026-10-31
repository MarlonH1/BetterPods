/*
    LibrePods - AirPods liberated from Apple’s ecosystem
    Copyright (C) 2025 LibrePods contributors

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

@file:OptIn(ExperimentalEncodingApi::class, ExperimentalMaterial3Api::class)

package me.kavishdevar.librepods.presentation.widgets

import android.app.PendingIntent
import android.appwidget.AppWidgetManager
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.util.SizeF
import android.view.View
import android.widget.RemoteViews
import androidx.compose.material3.ExperimentalMaterial3Api
import me.kavishdevar.librepods.MainActivity
import me.kavishdevar.librepods.R
import me.kavishdevar.librepods.bluetooth.AACPManager
import me.kavishdevar.librepods.bluetooth.BluetoothConnectionManager
import me.kavishdevar.librepods.data.AirPodsMax
import me.kavishdevar.librepods.data.Battery
import me.kavishdevar.librepods.data.BatteryComponent
import me.kavishdevar.librepods.data.BatteryStatus
import me.kavishdevar.librepods.data.Capability
import me.kavishdevar.librepods.services.AirPodsService
import me.kavishdevar.librepods.services.CompanionSessions
import me.kavishdevar.librepods.services.cleanDeviceName
import me.kavishdevar.librepods.services.ServiceManager
import kotlin.io.encoding.ExperimentalEncodingApi

/**
 * Builds the RemoteViews for both home-screen widgets from the live service state.
 * Widgets never show stale battery: while disconnected they switch to a connect prompt.
 */
object WidgetRenderer {
    const val ACTION_CONNECT = "me.kavishdevar.librepods.widget.CONNECT"
    const val ACTION_CYCLE_MODE = "me.kavishdevar.librepods.widget.CYCLE_MODE"
    const val ACTION_SET_MODE = "ACTION_SET_ANC_MODE"
    const val EXTRA_MODE = "ANC_MODE"

    private const val MODE_OFF = 1
    private const val MODE_ANC = 2
    private const val MODE_TRANSPARENCY = 3
    private const val MODE_ADAPTIVE = 4
    private const val LOW_BATTERY = 10
    private const val CAPSULE_LOW = 20

    /** Until this time the widgets admit they don't know the state yet instead of claiming "not connected". */
    @Volatile private var checkingUntil = 0L
    private const val CHECK_WINDOW_MS = 12_000L
    private val handler by lazy { android.os.Handler(android.os.Looper.getMainLooper()) }

    /** Mode the user just picked, per device ("" = main), shown until the AirPods confirm or it expires. */
    private val pendingModes = java.util.concurrent.ConcurrentHashMap<String, Pair<Int, Long>>()
    private const val PENDING_MS = 3_000L

    private fun pendingMode(mac: String?): Int? {
        val (mode, at) = pendingModes[mac ?: ""] ?: return null
        return mode.takeIf { android.os.SystemClock.elapsedRealtime() - at < PENDING_MS }
    }

    /**
     * Optimistic update: every surface (widgets, shade card, status notification) shows [mode]
     * immediately, before the command even reaches the AirPods.
     */
    fun setModeOptimistically(context: Context, mac: String?, mode: Int) {
        pendingModes[mac ?: ""] = mode to android.os.SystemClock.elapsedRealtime()
        refreshEverywhere(context)
        // If the AirPods reject or never answer, fall back to whatever they actually report.
        handler.postDelayed({ refreshEverywhere(context.applicationContext) }, PENDING_MS + 50)
    }

    private fun refreshEverywhere(context: Context) {
        // Shade first: it's what the user is looking at when they tap a card button.
        ServiceManager.getService()?.let { service ->
            runCatching { service.updateNotificationContent(true) }
        }
        updateAll(context)
    }

    /** The mode a surface should show for [mac]: a pending pick wins over the reported state. */
    fun displayedMode(mac: String?, actual: Int): Int = pendingMode(mac) ?: actual

    /** True while a connection attempt is running and the outcome isn't known yet. */
    fun isChecking(): Boolean = android.os.SystemClock.elapsedRealtime() < checkingUntil

    /** Call when a connection attempt starts (service start, connect tap, AirPods appearing). */
    fun markConnecting(context: Context) {
        checkingUntil = android.os.SystemClock.elapsedRealtime() + CHECK_WINDOW_MS
        val app = context.applicationContext
        handler.removeCallbacksAndMessages(null)
        handler.postDelayed({ updateAll(app) }, CHECK_WINDOW_MS + 100)
    }

    private data class State(
        val connected: Boolean,
        val loading: Boolean,
        val name: String,
        val isMax: Boolean,
        val left: Battery?,
        val right: Battery?,
        val case: Battery?,
        val mode: Int,
        val showOff: Boolean,
        val showAdaptive: Boolean,
        /** False when no AirPods were ever set up, so there's nothing to connect to. */
        val hasDevice: Boolean = true,
        /** Device this widget is pinned to; null follows the main device. */
        val mac: String? = null,
        val widgetId: Int = 0,
        /** Product art for the shade card; null falls back to the glyph. */
        val imageRes: Int? = null,
    )

    const val EXTRA_MAC = "me.kavishdevar.librepods.widget.MAC"
    private const val PREF_WIDGET_DEVICE = "widget_device_"

    /** The device a widget shows, or null for "Automatic" (the main device). */
    fun deviceFor(context: Context, widgetId: Int): String? =
        context.getSharedPreferences("settings", Context.MODE_PRIVATE).getString(PREF_WIDGET_DEVICE + widgetId, null)

    fun setDeviceFor(context: Context, widgetId: Int, mac: String?) {
        context.getSharedPreferences("settings", Context.MODE_PRIVATE).edit().apply {
            if (mac == null) remove(PREF_WIDGET_DEVICE + widgetId) else putString(PREF_WIDGET_DEVICE + widgetId, mac)
        }.apply()
    }

    fun forgetWidgets(context: Context, widgetIds: IntArray) {
        context.getSharedPreferences("settings", Context.MODE_PRIVATE).edit().apply {
            widgetIds.forEach { remove(PREF_WIDGET_DEVICE + it) }
        }.apply()
    }

    private fun isAclConnected(device: android.bluetooth.BluetoothDevice?) =
        device?.let { d -> runCatching { d.javaClass.getMethod("isConnected").invoke(d) as Boolean }.getOrDefault(true) } == true

    private fun listeningModeOf(commands: List<AACPManager.Companion.ControlCommandStatus>) =
        commands.find { it.identifier == AACPManager.Companion.ControlCommandIdentifiers.LISTENING_MODE }
            ?.value?.firstOrNull()?.toInt()?.takeIf { it in MODE_OFF..MODE_ADAPTIVE }

    private fun allowsOff(commands: List<AACPManager.Companion.ControlCommandStatus>) =
        commands.find { it.identifier == AACPManager.Companion.ControlCommandIdentifiers.ALLOW_OFF_OPTION }
            ?.value?.firstOrNull() == 0x01.toByte()

    private fun loading(context: Context, service: AirPodsService?): Boolean {
        // No service yet (process just started): we simply don't know — give it a moment.
        if (service == null && checkingUntil == 0L) markConnecting(context)
        return android.os.SystemClock.elapsedRealtime() < checkingUntil
    }

    private fun state(context: Context, mac: String? = null, widgetId: Int = 0): State {
        val service = ServiceManager.getService()
        val mainMac = service?.device?.address ?: service?.macAddress
        if (mac != null && mac != mainMac) return companionState(context, service, mac, widgetId)

        val prefs = context.getSharedPreferences("settings", Context.MODE_PRIVATE)
        val model = service?.airpodsInstance?.model
        val batteries = service?.batteryNotification?.getBattery().orEmpty()
            .filter { it.status != BatteryStatus.DISCONNECTED }
        val commands = runCatching { service?.aacpManager?.controlCommandStatusList?.toList() }.getOrNull().orEmpty()
        // The socket reports connected until closed, so also check the device's ACL link.
        val connected = service != null && BluetoothConnectionManager.aacpSocket?.isConnected == true &&
            isAclConnected(service.device)
        return State(
            connected = connected,
            loading = !connected && loading(context, service),
            name = prefs.getString("name", null) ?: model?.displayName ?: "AirPods",
            // Model is unknown until connected; the name still tells headphones from earbuds.
            isMax = model is AirPodsMax || (model == null && prefs.getString("name", null)?.contains("Max") == true),
            left = batteries.find { it.component == BatteryComponent.LEFT },
            right = batteries.find { it.component == BatteryComponent.RIGHT },
            case = batteries.find { it.component == BatteryComponent.CASE },
            // The status list also holds the value we sent last, so it's current even when the
            // device (e.g. AirPods Max) never pushes a listening-mode notification on connect.
            mode = pendingMode(null) ?: listeningModeOf(commands) ?: service?.ancNotification?.status ?: MODE_ANC,
            showOff = allowsOff(commands) || prefs.getBoolean("off_listening_mode", true),
            showAdaptive = model == null || Capability.ADAPTIVE_AUDIO in model.capabilities,
            hasDevice = connected || !(service?.macAddress ?: prefs.getString("mac_address", null)).isNullOrEmpty(),
            mac = mac,
            widgetId = widgetId,
            imageRes = model?.takeIf { it !is AirPodsMax }?.budsRes,
        )
    }

    @android.annotation.SuppressLint("MissingPermission")
    private fun companionState(context: Context, service: AirPodsService?, mac: String, widgetId: Int): State {
        val session = CompanionSessions.get(mac)
        val model = session?.instance?.model
        val batteries = session?.batteryNotification?.getBattery().orEmpty()
            .filter { it.status != BatteryStatus.DISCONNECTED }
        val commands = session?.aacpManager?.controlCommandStatusList?.toList().orEmpty()
        val bonded = runCatching {
            context.getSystemService(android.bluetooth.BluetoothManager::class.java).adapter.bondedDevices.find { it.address == mac }
        }.getOrNull()
        val connected = session != null && isAclConnected(session.device)
        return State(
            connected = connected,
            loading = !connected && loading(context, service),
            name = session?.name ?: runCatching { bonded?.name }.getOrNull()?.let(::cleanDeviceName) ?: model?.displayName ?: "AirPods",
            isMax = model is AirPodsMax || runCatching { bonded?.name }.getOrNull()?.contains("Max") == true,
            left = batteries.find { it.component == BatteryComponent.LEFT },
            right = batteries.find { it.component == BatteryComponent.RIGHT },
            case = batteries.find { it.component == BatteryComponent.CASE },
            mode = pendingMode(mac) ?: listeningModeOf(commands) ?: MODE_ANC,
            showOff = allowsOff(commands),
            showAdaptive = model == null || Capability.ADAPTIVE_AUDIO in model.capabilities,
            mac = mac,
            widgetId = widgetId,
            imageRes = model?.takeIf { it !is AirPodsMax }?.budsRes,
        )
    }

    // ---- Notification shade card (LSPosed) ----

    /**
     * Collapsed + expanded views for the shade card of [mac] (null = main device), or null while
     * that pair isn't connected. SystemUI renders them undecorated thanks to the module hook.
     */
    fun shadeCard(context: Context, mac: String?): Pair<RemoteViews, RemoteViews>? {
        val s = state(context, mac)
        if (!s.connected) return null
        val parts = if (s.isMax) listOfNotNull(s.left ?: s.right)
            else listOfNotNull(s.left, s.right, s.case)
        val status = listOf(
            context.getString(R.string.notif_connected),
            context.getString(modeLabel(s.mode)),
        ).joinToString(" · ")

        fun RemoteViews.header() {
            setTextViewText(R.id.nc_status, status)
            setTextViewText(R.id.nc_name, s.name)
            if (s.imageRes != null) {
                setImageViewResource(R.id.nc_image, s.imageRes)
                setViewVisibility(R.id.nc_image, View.VISIBLE)
                setViewVisibility(R.id.nc_glyph, View.GONE)
            } else {
                setImageViewResource(R.id.nc_glyph, if (s.isMax) R.drawable.ic_w_headphones else R.drawable.ic_w_earbuds)
                setViewVisibility(R.id.nc_image, View.GONE)
                setViewVisibility(R.id.nc_glyph, View.VISIBLE)
            }
        }

        val small = RemoteViews(context.packageName, R.layout.notif_card_small).apply {
            header()
            val lowest = parts.filter { it.component != BatteryComponent.CASE }.minByOrNull { it.level }
            setTextViewText(R.id.nc_summary_level, lowest?.let { "${it.level}%" } ?: "")
            if (lowest != null && lowest.level < CAPSULE_LOW && !lowest.isCharging())
                setTextColor(R.id.nc_summary_level, context.getColor(R.color.widget_error))
        }

        val big = RemoteViews(context.packageName, R.layout.notif_card).apply {
            header()
            val ids = listOf(
                Triple(R.id.nc_part0, R.id.nc_part0_level, R.id.nc_part0_label to R.id.nc_part0_wave),
                Triple(R.id.nc_part1, R.id.nc_part1_level, R.id.nc_part1_label to R.id.nc_part1_wave),
                Triple(R.id.nc_part2, R.id.nc_part2_level, R.id.nc_part2_label to R.id.nc_part2_wave),
            )
            ids.forEachIndexed { i, (container, level, rest) ->
                val b = parts.getOrNull(i)
                setViewVisibility(container, if (b != null) View.VISIBLE else View.GONE)
                if (b == null) return@forEachIndexed
                val low = b.level < CAPSULE_LOW && !b.isCharging()
                setTextViewText(level, "${b.level}%")
                setTextViewTextSize(level, android.util.TypedValue.COMPLEX_UNIT_SP, if (s.isMax) 30f else 20f)
                setTextColor(level, context.getColor(if (low) R.color.widget_error else R.color.widget_on_surface))
                val name = when {
                    s.isMax -> context.getString(R.string.battery)
                    b.component == BatteryComponent.LEFT -> context.getString(R.string.left)
                    b.component == BatteryComponent.RIGHT -> context.getString(R.string.right)
                    else -> context.getString(R.string.case_alt)
                }
                // Max has room for the word; buds columns are narrow, so they get a bolt.
                setTextViewText(rest.first,
                    if (b.isCharging() && s.isMax) "$name · ${context.getString(R.string.notif_charging)}" else name)
                setTextViewCompoundDrawablesRelative(rest.first, 0, 0,
                    if (b.isCharging() && !s.isMax) R.drawable.ic_w_bolt_small else 0, 0)
                val color = context.getColor(when {
                    low -> R.color.widget_error
                    b.isCharging() -> R.color.widget_tertiary
                    else -> R.color.widget_primary
                })
                setImageViewBitmap(rest.second, waveBitmap(context, b.level, color, if (s.isMax) 300 else 95))
            }

            val modes = buildList {
                if (s.showOff) add(MODE_OFF)
                add(MODE_TRANSPARENCY)
                if (s.showAdaptive) add(MODE_ADAPTIVE)
                add(MODE_ANC)
            }
            listOf(R.id.nc_mode0, R.id.nc_mode1, R.id.nc_mode2, R.id.nc_mode3).forEachIndexed { i, id ->
                val mode = modes.getOrNull(i)
                setViewVisibility(id, if (mode != null) View.VISIBLE else View.GONE)
                if (mode == null) return@forEachIndexed
                val on = mode == s.mode
                setInt(id, "setBackgroundResource", if (on) R.drawable.nc_mode_on else R.drawable.nc_mode_off)
                // Tint from here: SystemUI resolves our color resources against its own cache.
                setColorStateList(id, "setBackgroundTintList", android.content.res.ColorStateList.valueOf(
                    context.getColor(if (on) R.color.widget_primary else R.color.nc_mode_idle)))
                setImageViewResource(id, modeIcon(mode, filled = on))
                setInt(id, "setColorFilter", context.getColor(if (on) R.color.widget_on_primary else R.color.widget_on_surface_variant))
                setContentDescription(id, context.getString(modeLabel(mode)))
                setOnClickPendingIntent(id, PendingIntent.getBroadcast(
                    context, 300 + (mac?.hashCode() ?: 0) % 1000 * 8 + mode,
                    Intent(context, NoiseControlWidget::class.java).setAction(ACTION_SET_MODE)
                        .putExtra(EXTRA_MODE, mode).putExtra(EXTRA_MAC, mac)
                        // Foreground queue: background broadcasts can wait behind others for a while.
                        .addFlags(Intent.FLAG_RECEIVER_FOREGROUND),
                    PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
                ))
            }
        }
        return small to big
    }

    /** M3 Expressive wavy progress: wavy active part, a gap, then a flat track. */
    private fun waveBitmap(context: Context, level: Int, color: Int, widthDp: Int): android.graphics.Bitmap {
        val d = context.resources.displayMetrics.density
        val w = (widthDp * d).toInt()
        val h = (12 * d).toInt()
        val bmp = android.graphics.Bitmap.createBitmap(w, h, android.graphics.Bitmap.Config.ARGB_8888)
        val c = android.graphics.Canvas(bmp)
        val stroke = 3 * d
        val paint = android.graphics.Paint(android.graphics.Paint.ANTI_ALIAS_FLAG).apply {
            style = android.graphics.Paint.Style.STROKE
            strokeWidth = stroke
            strokeCap = android.graphics.Paint.Cap.ROUND
        }
        val mid = h / 2f
        val start = stroke / 2
        val end = w - stroke / 2
        val split = start + (end - start) * level.coerceIn(0, 100) / 100f
        val gap = 4 * d
        if (split + gap < end) {
            paint.color = context.getColor(R.color.widget_outline_variant)
            c.drawLine(minOf(end, split + gap), mid, end, mid, paint)
        }
        paint.color = color
        val amp = (h - stroke) / 2f * 0.8f
        val wavelength = 14 * d
        val path = android.graphics.Path().apply {
            moveTo(start, mid)
            var x = start
            while (x <= split) {
                lineTo(x, mid + amp * kotlin.math.sin((x - start) / wavelength * 2 * Math.PI).toFloat())
                x += d / 2
            }
        }
        c.drawPath(path, paint)
        return bmp
    }

    fun updateAll(context: Context) {
        val manager = AppWidgetManager.getInstance(context)
        fun each(provider: Class<*>, build: (Context, State) -> RemoteViews) {
            manager.getAppWidgetIds(ComponentName(context, provider)).forEach { id ->
                manager.updateAppWidget(id, build(context, state(context, deviceFor(context, id), id)))
            }
        }
        each(BatteryWidget::class.java, ::airPodsViews)
        each(NoiseControlWidget::class.java, ::modeViews)
        each(BatteryWidgetSmall::class.java, ::airPodsSmall)
        each(NoiseControlWidgetGrid::class.java, ::modeGrid)
    }

    /** Renders the given widgets of [provider]; used by the providers' onUpdate. */
    fun update(context: Context, manager: AppWidgetManager, provider: Class<*>, ids: IntArray) {
        ids.forEach { id ->
            val s = state(context, deviceFor(context, id), id)
            manager.updateAppWidget(id, when (provider) {
                BatteryWidget::class.java -> airPodsViews(context, s)
                NoiseControlWidget::class.java -> modeViews(context, s)
                BatteryWidgetSmall::class.java -> airPodsSmall(context, s)
                else -> modeGrid(context, s)
            })
        }
    }

    private fun modeGrid(context: Context, s: State) =
        modes(context, s, R.layout.widget_modes_grid, R.drawable.w_mode_on, R.drawable.w_mode_off)

    private fun airPodsViews(context: Context, s: State) = RemoteViews(
        mapOf(
            SizeF(110f, 110f) to airPodsSmall(context, s),
            SizeF(250f, 130f) to airPodsLarge(context, s),
        )
    )

    private fun modeViews(context: Context, s: State) = RemoteViews(
        mapOf(
            SizeF(110f, 110f) to modeGrid(context, s),
            SizeF(250f, 48f) to modes(context, s, R.layout.widget_modes_strip, R.drawable.w_strip_on, R.drawable.w_strip_off),
        )
    )

    private fun airPodsLarge(context: Context, s: State) =
        RemoteViews(context.packageName, R.layout.widget_airpods_large).apply {
            setConnectedState(context, s)
            setTextViewText(R.id.w_name, s.name)
            setTextViewText(R.id.w_name_disconnected, if (s.hasDevice) s.name else context.getString(R.string.widget_no_airpods))
            setTextViewText(R.id.w_disconnected_subtitle, context.getString(R.string.widget_no_airpods_hint))
            setViewVisibility(R.id.w_disconnected_subtitle, if (s.hasDevice) View.GONE else View.VISIBLE)
            setRestGlyph(context, s)
            setImageViewResource(R.id.w_device_image, if (s.isMax) R.drawable.ic_w_headphones else R.drawable.ic_w_earbuds)
            // Connecting: same layout as connected, with the batteries still to come.
            setViewVisibility(R.id.w_skeleton, if (s.loading) View.VISIBLE else View.GONE)
            setViewVisibility(R.id.w_mode_button, if (s.loading) View.GONE else View.VISIBLE)
            if (s.loading) {
                setTextViewText(R.id.w_mode_label, context.getString(R.string.widget_connecting))
                listOf(R.id.w_left, R.id.w_right, R.id.w_case).forEach { setViewVisibility(it, View.GONE) }
            }
            if (!s.connected) return@apply

            setTextViewText(R.id.w_mode_label, context.getString(modeLabel(s.mode)))
            setImageViewResource(R.id.w_mode_button, modeIcon(s.mode, filled = true))
            setOnClickPendingIntent(R.id.w_mode_button, broadcast(context, s, ACTION_CYCLE_MODE, 10))

            if (s.isMax) {
                // AirPods Max: one battery, no case
                setCapsule(R.id.w_left, R.id.w_left_progress, R.id.w_left_level, s.left ?: s.right, minFill = 25)
                setTextViewText(R.id.w_left_label, context.getString(R.string.battery))
                setViewVisibility(R.id.w_right, View.GONE)
                setViewVisibility(R.id.w_case, View.GONE)
            } else {
                setTextViewText(R.id.w_left_label, context.getString(R.string.left))
                setCapsule(R.id.w_left, R.id.w_left_progress, R.id.w_left_level, s.left, minFill = 60)
                setCapsule(R.id.w_right, R.id.w_right_progress, R.id.w_right_level, s.right, minFill = 60)
                setCapsule(R.id.w_case, R.id.w_case_progress, R.id.w_case_level, s.case, minFill = 60)
            }
        }

    private fun airPodsSmall(context: Context, s: State) =
        RemoteViews(context.packageName, R.layout.widget_airpods_small).apply {
            setConnectedState(context, s)
            setRestLabels(context, s)
            setViewVisibility(R.id.w_ring, if (s.loading) View.GONE else View.VISIBLE)
            setViewVisibility(R.id.w_ring_spinner, if (s.loading) View.VISIBLE else View.GONE)
            setViewVisibility(R.id.w_ring_level, if (s.loading) View.GONE else View.VISIBLE)
            if (s.loading) {
                setImageViewResource(R.id.w_ring_icon, if (s.isMax) R.drawable.ic_w_headphones else R.drawable.ic_w_earbuds)
                setColorStateList(R.id.w_ring_icon, "setImageTintList", R.color.widget_primary)
                setTextViewText(R.id.w_ring_caption, s.name)
                setColorStateList(R.id.w_ring_caption, "setTextColor", R.color.widget_on_surface_variant)
            }
            if (!s.connected) return@apply

            val main = listOfNotNull(s.left, s.right).minByOrNull { it.level }
            val level = main?.level ?: 0
            val low = main != null && level <= LOW_BATTERY && !main.isCharging()
            setProgressBar(R.id.w_ring, 100, level, false)
            setColorStateList(R.id.w_ring, "setProgressTintList", if (low) R.color.widget_error else R.color.widget_primary)
            setTextViewText(R.id.w_ring_level, main?.let { "${it.level}%" } ?: "–")
            setImageViewResource(
                R.id.w_ring_icon,
                when {
                    low -> R.drawable.ic_w_battery_alert
                    main?.isCharging() == true -> R.drawable.ic_w_bolt_fill
                    s.isMax -> R.drawable.ic_w_headphones
                    else -> R.drawable.ic_w_earbuds
                }
            )
            setColorStateList(R.id.w_ring_icon, "setImageTintList", if (low) R.color.widget_error else R.color.widget_primary)
            val caption = when {
                low -> context.getString(R.string.widget_charge_soon)
                s.isMax -> s.name
                s.case != null -> context.getString(R.string.widget_case_percent, s.case.level)
                else -> s.name
            }
            setTextViewText(R.id.w_ring_caption, caption)
            setColorStateList(R.id.w_ring_caption, "setTextColor", if (low) R.color.widget_error else R.color.widget_on_surface_variant)
        }

    private fun modes(context: Context, s: State, layout: Int, onBg: Int, offBg: Int) =
        RemoteViews(context.packageName, layout).apply {
            setConnectedState(context, s)
            if (layout == R.layout.widget_modes_grid) {
                setRestLabels(context, s)
            }
            if (layout == R.layout.widget_modes_strip) {
                setTextViewText(
                    R.id.w_strip_disconnected_label,
                    if (s.hasDevice) s.name else context.getString(R.string.widget_no_airpods)
                )
                setRestGlyph(context, s)
                val level = listOfNotNull(s.left, s.right).minOfOrNull { it.level }
                setTextViewText(R.id.w_strip_battery, level?.let { "$it%" } ?: "")
                setTextViewCompoundDrawablesRelative(
                    R.id.w_strip_battery,
                    if (s.isMax) R.drawable.ic_w_headphones else R.drawable.ic_w_earbuds, 0, 0, 0
                )
            }
            if (!s.connected && !s.loading) return@apply

            val buttons = listOf(
                Triple(R.id.w_mode_off, MODE_OFF, s.showOff),
                Triple(R.id.w_mode_transparency, MODE_TRANSPARENCY, true),
                Triple(R.id.w_mode_adaptive, MODE_ADAPTIVE, s.showAdaptive),
                Triple(R.id.w_mode_anc, MODE_ANC, true),
            )
            for ((id, mode, visible) in buttons) {
                setViewVisibility(id, if (visible) View.VISIBLE else View.GONE)
                // Connecting: the buttons are there but dimmed and inert until we know the mode.
                setFloat(id, "setAlpha", if (s.loading) 0.38f else 1f)
                val selected = s.connected && s.mode == mode
                if (s.loading) {
                    setInt(id, "setBackgroundResource", offBg)
                    setImageViewResource(id, modeIcon(mode, filled = false))
                    setColorStateList(id, "setImageTintList", R.color.widget_on_surface_variant)
                    setOnClickPendingIntent(id, null)
                    continue
                }
                setInt(id, "setBackgroundResource", if (selected) onBg else offBg)
                setImageViewResource(id, modeIcon(mode, filled = selected))
                setColorStateList(id, "setImageTintList", if (selected) R.color.widget_on_primary else R.color.widget_on_surface_variant)
                setOnClickPendingIntent(
                    id,
                    PendingIntent.getBroadcast(
                        context, s.widgetId * 32 + mode,
                        Intent(context, NoiseControlWidget::class.java).setAction(ACTION_SET_MODE)
                            .putExtra(EXTRA_MODE, mode).putExtra(EXTRA_MAC, s.mac),
                        PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
                    )
                )
            }
        }

    /** Resting (not connected) state: the device glyph, or a "+" when nothing is set up yet. */
    private fun RemoteViews.setRestGlyph(context: Context, s: State) {
        setImageViewResource(R.id.w_rest_glyph, if (s.isMax) R.drawable.ic_w_headphones else R.drawable.ic_w_earbuds)
        setImageViewResource(R.id.w_connect_button, if (s.hasDevice) R.drawable.ic_w_bluetooth_fill else R.drawable.ic_w_add)
        setContentDescription(R.id.w_connect_button, context.getString(if (s.hasDevice) R.string.widget_connect else R.string.widget_set_up))
    }

    private fun RemoteViews.setRestLabels(context: Context, s: State) {
        setImageViewResource(R.id.w_rest_glyph, if (s.isMax) R.drawable.ic_w_headphones else R.drawable.ic_w_earbuds)
        setTextViewText(R.id.w_disconnected_label, if (s.hasDevice) s.name else context.getString(R.string.widget_no_airpods))
        // The empty ring already says "not here"; only an unset widget needs a hint.
        setTextViewText(R.id.w_disconnected_sub, context.getString(R.string.widget_set_up))
        setViewVisibility(R.id.w_disconnected_sub, if (s.hasDevice) View.GONE else View.VISIBLE)
    }

    /** Toggles between the connected (or connecting) and disconnected containers and wires taps. */
    private fun RemoteViews.setConnectedState(context: Context, s: State) {
        val content = s.connected || s.loading
        setViewVisibility(R.id.w_connected, if (content) View.VISIBLE else View.GONE)
        setViewVisibility(R.id.w_disconnected, if (content) View.GONE else View.VISIBLE)
        val openApp = PendingIntent.getActivity(
            context, 0, Intent(context, MainActivity::class.java),
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )
        // Nothing set up yet: tapping opens the app instead of "connecting" to nothing.
        val connect = if (s.hasDevice) broadcast(context, s, ACTION_CONNECT, 11) else openApp
        setOnClickPendingIntent(android.R.id.background, if (content) openApp else connect)
        setOnClickPendingIntent(R.id.w_disconnected, connect)
        if (!content) setOnClickPendingIntent(R.id.w_connect_button, connect)
    }

    /**
     * [minFill] keeps the filled pill wide enough to hold the "82% / Left" text: it never
     * shrinks below that, and turns red below 20 % instead.
     */
    private fun RemoteViews.setCapsule(container: Int, progress: Int, label: Int, battery: Battery?, minFill: Int) {
        setViewVisibility(container, if (battery != null) View.VISIBLE else View.GONE)
        if (battery == null) return
        val low = battery.level < CAPSULE_LOW && !battery.isCharging()
        setProgressBar(progress, 100, maxOf(minFill, battery.level), false)
        // The text box spans exactly the minimum pill width, so the text sits centered in it.
        setFloat(textRow(container), "setWeightSum", 100f / minFill)
        val fill = when {
            low -> R.color.widget_error_container
            battery.component == BatteryComponent.CASE -> R.color.widget_tertiary_container
            else -> R.color.widget_primary_container
        }
        setColorStateList(progress, "setProgressTintList", fill)
        setTextViewText(label, "${battery.level}%")
        setTextViewCompoundDrawablesRelative(label, 0, 0, if (battery.isCharging()) R.drawable.ic_w_bolt_small else 0, 0)
    }

    private fun textRow(container: Int) = when (container) {
        R.id.w_left -> R.id.w_left_text_row
        R.id.w_right -> R.id.w_right_text_row
        else -> R.id.w_case_text_row
    }

    private fun broadcast(context: Context, s: State, action: String, requestCode: Int) =
        PendingIntent.getBroadcast(
            context, s.widgetId * 32 + requestCode,
            Intent(context, BatteryWidget::class.java).setAction(action).putExtra(EXTRA_MAC, s.mac),
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )

    private fun Battery.isCharging() =
        status == BatteryStatus.CHARGING || status == BatteryStatus.OPTIMIZED_CHARGING

    private fun modeLabel(mode: Int) = when (mode) {
        MODE_OFF -> R.string.widget_noise_control_off
        MODE_TRANSPARENCY -> R.string.transparency
        MODE_ADAPTIVE -> R.string.adaptive
        else -> R.string.noise_cancellation
    }

    private fun modeIcon(mode: Int, filled: Boolean) = when (mode) {
        MODE_OFF -> if (filled) R.drawable.ic_w_noise_control_off_fill else R.drawable.ic_w_noise_control_off
        MODE_TRANSPARENCY -> if (filled) R.drawable.ic_w_hearing_fill else R.drawable.ic_w_hearing
        MODE_ADAPTIVE -> if (filled) R.drawable.ic_w_auto_awesome_fill else R.drawable.ic_w_auto_awesome
        else -> if (filled) R.drawable.ic_w_noise_control_on_fill else R.drawable.ic_w_noise_control_on
    }

    /** Handles widget taps that need the running service. Socket I/O runs off the main thread. */
    fun handleAction(context: Context, intent: Intent, onDone: () -> Unit) {
        val service: AirPodsService? = ServiceManager.getService()
        val mac = intent.getStringExtra(EXTRA_MAC)
        val mainMac = service?.device?.address ?: service?.macAddress
        val companion = if (mac != null && mac != mainMac) CompanionSessions.get(mac) else null
        when (intent.action) {
            ACTION_CONNECT -> {
                markConnecting(context)
                updateAll(context)
                when {
                    service == null -> context.startActivity(Intent(context, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
                    mac != null -> service.connectDevice(mac)
                    else -> service.reconnectFromSavedMac()
                }
            }
            ACTION_CYCLE_MODE -> if (companion != null) {
                val commands = companion.aacpManager.controlCommandStatusList.toList()
                val configs = commands.find { it.identifier == AACPManager.Companion.ControlCommandIdentifiers.LISTENING_MODE_CONFIGS }
                    ?.value?.firstOrNull()?.toInt() ?: 0x0E
                val current = pendingMode(mac) ?: listeningModeOf(commands) ?: MODE_ANC
                val next = me.kavishdevar.librepods.services.getNextMode(current, configs, allowsOff(commands))
                setModeOptimistically(context, mac, next)
                sendMode(context, companion.aacpManager, next, onDone)
                return
            } else {
                context.sendBroadcast(Intent("me.kavishdevar.librepods.SET_ANC_MODE").setPackage(context.packageName))
            }
            ACTION_SET_MODE -> {
                val mode = intent.getIntExtra(EXTRA_MODE, MODE_ANC)
                if (companion != null) {
                    setModeOptimistically(context, mac, mode)
                    sendMode(context, companion.aacpManager, mode, onDone)
                    return
                }
                if (service != null) {
                    // Show the new mode right away; the AirPods' confirmation re-renders it anyway.
                    service.ancNotification.setStatus(byteArrayOf(mode.toByte()))
                    setModeOptimistically(context, null, mode)
                    sendMode(context, service.aacpManager, mode, onDone)
                    return
                }
            }
        }
        onDone()
    }

    /** Socket I/O off the main thread; the status list updates as the command is sent. */
    private fun sendMode(context: Context, aacp: AACPManager, mode: Int, onDone: () -> Unit) {
        Thread {
            runCatching {
                aacp.sendControlCommand(AACPManager.Companion.ControlCommandIdentifiers.LISTENING_MODE.value, mode.toByte())
            }
            updateAll(context)
            onDone()
        }.start()
    }
}
