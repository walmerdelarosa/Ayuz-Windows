# Ayuz en Windows

Esta rama porta Ayuz a Windows 10/11 (x64) conservando la compatibilidad con Linux. La interfaz es la misma (GTK4 + libadwaita); lo que cambia es el backend de hardware:

| En Linux | En Windows |
| --- | --- |
| `asusd` / `supergfxctl` por D-Bus | Driver **ATKACPI** de ASUS (`\\.\ATKACPI`, métodos `DSTS`/`DEVS`) |
| Aura RGB vía `asusd` | Colección HID del teclado ASUS (página `0xFF31`, reporte `0x5D`); en TUF, ATKACPI |
| Bandeja `ksni` (StatusNotifierItem) | Icono nativo del área de notificación (Win32) |
| Autoinicio `~/.config/autostart` | `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` |
| sysfs / procfs | Registro (SMBIOS), `GetSystemPowerStatus`, `GlobalMemoryStatusEx`, sensor de temperatura del EC |

## Requisitos

- Un portátil ASUS con el driver **ASUS System Control Interface** instalado. Viene con MyASUS y también se descarga suelto desde la página de soporte del modelo (sección *Software y utilidades*). Sin él, las funciones de hardware aparecen desactivadas con un aviso.
- No hace falta ejecutar como administrador.

## Instalación (zip portable)

1. Descarga `ayuz-windows-x86_64.zip` desde la pestaña **Actions** (artefacto del flujo *Build*) o desde **Releases** si publicaste una etiqueta `v*`.
2. Descomprime en una carpeta fija, por ejemplo `C:\Programas\Ayuz`.
3. Ejecuta `bin\ayuz.exe`. Para que arranque con Windows, activa **Iniciar con el sistema** en *Sistema → Ajustes de la app* (se inicia oculto en la bandeja).

La configuración se guarda en `%APPDATA%\ayuz\config\`.

## Qué funciona

| Función | Estado en Windows |
| --- | --- |
| Perfil de ventilador (Equilibrado / Rendimiento / Silencioso) | ✅ ATKACPI; detecta la variante Vivobook/Zenbook |
| Límite de carga 80 % / 100 % | ✅ ATKACPI; Ayuz lo vuelve a aplicar al iniciar, porque el firmware lo olvida al reiniciar |
| Modo de GPU Eco / Estándar / Ultimate (MUX) | ✅ ATKACPI; el cambio de MUX requiere reiniciar |
| Bloqueo de la tecla Fn | ✅ ATKACPI, se aplica al instante |
| Aura RGB (modo, colores, velocidad, dirección, brillo) | ✅ HID; TUF por ATKACPI. Modos: estático, respiración, ciclo, arcoíris, pulso |
| Página de inicio (modelo, placa, BIOS, versión de Windows, número de serie, batería, CPU, temperatura, RAM, disco) | ✅ |
| Bandeja, instancia única y autoinicio | ✅ Si abres Ayuz otra vez, se muestra la ventana ya abierta |
| Perfiles de Ayuz | ✅ |

## Qué no está disponible (todavía)

Estas secciones se ven en gris en Windows porque dependen de herramientas exclusivas de Linux:

- **Pantalla**: atenuación OLED, gama de color, modo objetivo, cuidado OLED (KDE / `kscreen-doctor`). En Windows lo gestiona MyASUS o el panel de color del sistema.
- **Audio**: refuerzo de volumen y perfiles de EasyEffects (PipeWire).
- **Teclado**: brillo automático por sensor de luz y apagado por inactividad (`iio-sensor-proxy`, `swayidle`). El atajo FN+F para ciclar perfiles depende del software de ASUS instalado en Windows.
- **Touchpad**: gestos en los bordes, NumberPad, desactivar al escribir (evdev/uinput). En Windows el driver del touchpad de precisión ya los ofrece.
- **AniMatrix**, **memoria de la APU** y los estados de energía de Aura por zona.

## Convivencia con MyASUS, Armoury Crate o G-Helper

Todas estas aplicaciones escriben en el mismo firmware. Si dos programas controlan la misma función (por ejemplo, el perfil de ventilador o el límite de carga), el último que escriba gana y pueden "pelearse" al iniciar. Lo recomendable es dejar cada función a una sola aplicación.

## Precauciones

- **Modo Eco** apaga la GPU dedicada. Si un programa la está usando en ese momento, puede cerrarse o el cambio puede fallar; cierra juegos y editores antes de cambiar.
- **Ultimate (MUX)** cambia la salida de la pantalla interna y solo se aplica tras reiniciar. Para volver a Eco desde Ultimate: elige Estándar, reinicia y después Eco.
- El modo Aura se envía con el comando estándar de ASUS. En teclados con firmware poco común algún modo puede verse distinto; si un modo no responde, prueba el estático.

## Compilar desde el código fuente

1. Instala [MSYS2](https://www.msys2.org/) y abre la terminal **UCRT64**.
2. Instala las dependencias:

   ```bash
   pacman -S --needed zip mingw-w64-ucrt-x86_64-{rust,gcc,pkgconf,gtk4,libadwaita,librsvg,adwaita-icon-theme}
   ```

3. Compila y empaqueta:

   ```bash
   cargo build --release
   bash packaging/windows/bundle.sh   # genera dist/ayuz-windows-x86_64.zip
   ```

   Para depurar, `cargo run` desde la misma terminal funciona sin empaquetar (las DLL de GTK se toman de MSYS2).

## Notas técnicas

- `src/platform/windows/` contiene todo el código específico de Windows, con enlaces Win32 escritos a mano (sin crates adicionales):
  - `atkacpi.rs`: acceso al driver; los ID de dispositivo son los mismos que usa el módulo `asus-wmi` del kernel de Linux.
  - `asus.rs`: ventilador, carga, GPU, Fn-lock y temperatura.
  - `aura.rs` + `hid.rs`: iluminación por HID o ATKACPI (TUF).
  - `sysinfo.rs`, `tray.rs`, `autostart.rs`, `single_instance.rs`, `state.rs`.
- `src/services/dbus.rs` mantiene la misma API pública en ambas plataformas; en Windows reexporta las funciones de `platform::windows`, así que los componentes de la interfaz no cambian.
- Los servicios basados en evdev tienen sustitutos inertes en `src/services/unsupported/`.
- Los textos que mencionan herramientas de Linux tienen una variante `*_win` en `locales/*.yml`, seleccionada con `crate::platform::pk()`.
- Valores que el firmware no devuelve (límite de carga, último perfil de ventilador) se guardan en `%APPDATA%\ayuz\config\windows_state.txt`.
