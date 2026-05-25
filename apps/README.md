# Nodalix Native Apps

`apps/` contains the native and first-party application ecosystem for Nodalix OS.

The long-term goal is a coherent Linux-native desktop suite: system shell components, productivity tools, media tools, engineering tools, and developer tooling that share one visual language and safety model.

## Categories

- System apps: shell, settings, files, notifications, updates, capture, power, network, audio, Bluetooth, and OS onboarding.
- Productivity apps: documents, spreadsheets, presentations, notes, drawing, and local transfer.
- Engineering apps: CAD, scientific tools, plotting, unit conversion, airfoil, structures, and CFD research.
- Media apps: photo and video tools.
- Developer apps: Nodalix DevKit and Bundle Creator.

## Development conventions

- Each app should have `README.md`, `ROADMAP.md`, `SPEC.md`, and `STATUS.md`.
- Rust + GTK4/libadwaita is preferred for native system apps.
- Tauri is acceptable where already established, such as `nodalix-files`.
- Prototypes must not modify system configuration automatically.
- Destructive actions require confirmation.
- Wrappers live in `local/bin`.
- Shared OS configuration lives in `config/`.
- Build artifacts such as `target/`, `node_modules/`, `dist/`, and `build/` are not part of the source contract.

## Planned ecosystem

```text
apps/
├─ native-app-template
├─ nodalix-greeter
├─ nodalix-lock
├─ nodalix-settings
├─ nodalix-control-center
├─ nodalix-bar
├─ nodalix-dock
├─ nodalix-command-bar
├─ nodalix-files
├─ nodalix-notifications
├─ nodalix-wallpapers
├─ nodalix-welcome
├─ nodalix-updater
├─ nodalix-store
├─ nodalix-monitor
├─ nodalix-capture
├─ nodalix-audio
├─ nodalix-network
├─ nodalix-bluetooth
├─ nodalix-power
├─ nodalix-tweaks
├─ nodalix-bundle-creator
├─ nodalix-pages
├─ nodalix-cells
├─ nodalix-point
├─ nodalix-drop
├─ nodalix-photo
├─ nodalix-video
├─ nodalix-draw
├─ nodalix-notes
├─ nodalix-cad
├─ nodalix-lab
├─ nodalix-plot
├─ nodalix-units
├─ nodalix-airfoil
├─ nodalix-structures
├─ nodalix-cfd
└─ nodalix-devkit
```

## Current implementation model

Existing apps are preserved. New apps start as documented prototypes or minimal Rust/GTK skeletons. Nothing in this directory should replace Waybar, modify Hyprland autostart, edit greetd, or install services without an explicit integration step.

# Lix Apps — Ecosistema nativo de Nodalix OS

Este documento define una propuesta unificada para las aplicaciones propias de **Nodalix OS**, agrupando herramientas equivalentes de Windows, Adobe, Autodesk, MATLAB, Office y software científico/ingenieril en una suite coherente bajo la marca **Lix**.

La idea no es crear una app por cada programa existente, sino crear **apps Lix potentes por categoría**, con módulos internos.

---

## Principio de diseño

En vez de tener decenas de aplicaciones separadas:

- Adobe Premiere
- DaVinci Resolve
- After Effects
- Media Encoder

se agrupan en una sola app:

- **LixStudio**

Y dentro de ella habría módulos:

- Edición de vídeo
- Color
- Motion graphics
- VFX
- Render/export

Este mismo criterio se aplica al resto del ecosistema.

---

# 1. Sistema Nodalix

| App Lix | Sustituye / agrupa | Función |
|---|---|---|
| **LixFiles** | Windows Explorer, Finder, Nautilus, Dolphin | Gestor de archivos nativo |
| **LixSettings** | Windows Settings, GNOME Settings, KDE Settings | Ajustes generales del sistema |
| **LixControl Center** | macOS Control Center, Windows Quick Settings | Centro rápido de controles |
| **LixStore** | Microsoft Store, App Store, Discover | Tienda e instalador de apps |
| **LixUpdater** | Windows Update, Discover Updates | Actualizaciones del sistema |
| **LixGuard** | Windows Defender, firewall GUIs | Seguridad, firewall y privacidad |
| **LixDrop** | AirDrop, LocalSend, Phone Link | Transferencia rápida de archivos |
| **LixCapture** | Snipping Tool, ShareX, OBS básico | Capturas y grabación de pantalla |
| **LixWelcome** | Welcome apps, setup assistants | Bienvenida y configuración inicial |
| **LixGreeter** | SDDM, GDM, Windows Login | Pantalla de inicio de sesión |
| **LixLock** | Windows Lock Screen, Hyprlock | Pantalla de bloqueo |
| **LixBar** | Waybar, taskbar, top bar | Barra superior del sistema |
| **LixDock** | macOS Dock, taskbar | Dock de aplicaciones |
| **LixNotifications** | Notification Center | Centro de notificaciones |
| **LixTerminal** | Windows Terminal, Ghostty, Alacritty | Terminal nativa |
| **LixTweaks** | PowerToys, herramientas avanzadas | Ajustes avanzados del sistema |

---

# 2. Productividad y documentación

| App Lix | Sustituye / agrupa | Módulos internos |
|---|---|---|
| **LixOffice** | Microsoft Office, LibreOffice, iWork, Google Docs | Hub general de oficina |
| **LixPages** | Microsoft Word, Apple Pages, LibreOffice Writer | Documentos de texto |
| **LixNumbers** | Microsoft Excel, Apple Numbers, LibreOffice Calc | Hojas de cálculo |
| **LixSlides** | PowerPoint, Keynote, LibreOffice Impress | Presentaciones |
| **LixPDF** | Adobe Acrobat, PDF-XChange, Preview | PDF, anotaciones, firmas |
| **LixNotes** | OneNote, Notion, Obsidian, Apple Notes | Notas y base de conocimiento |
| **LixCite** | Zotero, Mendeley, EndNote | Bibliografía y citas |
| **LixProject** | Microsoft Project, Planner, Trello, Jira básico | Gestión de proyectos |
| **LixDiagram** | Microsoft Visio, Draw.io, Lucidchart | Diagramas técnicos |
| **LixBoard** | Miro, Microsoft Whiteboard | Pizarra visual |
| **LixProof** | Grammarly, LanguageTool | Corrección y revisión de textos |

---

# 3. Creatividad, multimedia y diseño

| App Lix | Sustituye / agrupa | Módulos internos |
|---|---|---|
| **LixStudio** | Adobe Premiere, DaVinci Resolve, After Effects, Media Encoder | Vídeo, color, VFX, motion y render |
| **LixPhoto** | Photoshop, Lightroom, Affinity Photo, Camera RAW | Foto, RAW, retoque y biblioteca |
| **LixDesign** | Illustrator, Figma, Adobe XD, Affinity Designer | Vectorial, UI/UX, iconos y prototipos |
| **LixLayout** | InDesign, Publisher, Scribus | Maquetación editorial |
| **LixAudio** | Audition, Audacity, Logic básico | Grabación, edición y mezcla de audio |
| **Lix3D** | Blender, Cinema4D, Substance Painter | Modelado 3D, materiales, render y animación |
| **LixAnimate** | Adobe Animate, Toon Boom básico | Animación 2D |
| **LixCreate** | Canva, Adobe Express | Diseño rápido para documentos y redes |

---

# 4. CAD, modelado e ingeniería gráfica

| App Lix | Sustituye / agrupa | Módulos internos |
|---|---|---|
| **LixCAD** | AutoCAD, BricsCAD, DraftSight, LibreCAD | CAD 2D/3D general, DWG, DXF, STEP |
| **LixModel** | Fusion 360, Inventor, SolidWorks, Creo, FreeCAD | Modelado paramétrico mecánico |
| **LixBIM** | Revit, Archicad, Navisworks | Arquitectura, BIM y coordinación |
| **LixCivil** | Civil 3D, InfraWorks, Global Mapper civil | Obra civil, carreteras y topografía |
| **LixStructures** | Tekla, Robot, SAP2000, CYPE estructuras | Estructuras metálicas y hormigón |
| **LixSurface** | Rhino, Grasshopper, Alias | Superficies, NURBS y diseño paramétrico |
| **LixDraft** | AutoCAD LT, LibreCAD | Dibujo técnico 2D ligero |
| **LixMesh** | HyperMesh, Salome, Gmsh | Mallado y preprocesado CAE |

---

# 5. Matemáticas, ciencia y simulación

| App Lix | Sustituye / agrupa | Módulos internos |
|---|---|---|
| **LixMath** | MATLAB, Mathematica, Maple, Mathcad, Octave | Cálculo numérico, simbólico, notebooks y scripts |
| **LixSim** | Simulink, LabVIEW visual, Modelica | Simulación por bloques y sistemas dinámicos |
| **LixPlot** | OriginPro, Desmos, GeoGebra, GraphPad Prism | Gráficas, análisis de datos y ajuste de curvas |
| **LixLab** | LabVIEW, ELN, instrumentación | Laboratorio, adquisición de datos y cuaderno técnico |
| **LixUnits** | Conversores de unidades, tablas técnicas | Unidades, constantes y propiedades físicas |
| **LixNotebook** | Jupyter, Spyder, RStudio | Notebooks científicos Python/R/Julia |
| **LixData** | Power BI, Tableau, Orange, Weka | BI, datasets y machine learning visual |

---

# 6. CAE, CFD, FEA y aeroespacial

| App Lix | Sustituye / agrupa | Módulos internos |
|---|---|---|
| **LixCAE** | ANSYS Workbench, Abaqus, Nastran, COMSOL | Simulación general CAE y multifísica |
| **LixCFD** | ANSYS Fluent, CFX, OpenFOAM, SU2 | Dinámica de fluidos computacional |
| **LixFEA** | ANSYS Mechanical, Abaqus, Nastran | Elementos finitos estructurales |
| **LixAero** | XFLR5, AVL, SU2 Aero | Aeronaves, alas, estabilidad y polares |
| **LixAirfoil** | XFOIL, JavaFoil, Profili | Perfiles aerodinámicos |
| **LixDynamics** | Adams, LS-DYNA, dinámica multicuerpo | Dinámica, impacto y sistemas multicuerpo |
| **LixPrePost** | Salome-Meca, ParaView, PrePoMax | Preprocesado y postprocesado |

---

# 7. Electrónica, control e IoT

| App Lix | Sustituye / agrupa | Módulos internos |
|---|---|---|
| **LixCircuit** | KiCad, Altium, Proteus, Multisim, LTspice | PCB, esquemas y simulación electrónica |
| **LixControlSim** | TIA Portal, Codesys, Simulink Control | PLC, control automático y automatización |
| **LixRobotics** | ROS tools, Gazebo, RViz | Robótica, sensores y simulación |
| **LixIoT** | Home Assistant tools, Node-RED, MQTT tools | IoT, domótica y automatización |
| **LixElectrical** | AutoCAD Electrical, EPLAN | Esquemas eléctricos industriales |

---

# 8. Desarrollo, datos e inteligencia artificial

| App Lix | Sustituye / agrupa | Módulos internos |
|---|---|---|
| **LixCode** | VS Code, Zed, JetBrains, Visual Studio | Editor/IDE |
| **LixDevkit** | GitHub Desktop, Docker Desktop, Postman, DBeaver | Git, contenedores, APIs y bases de datos |
| **LixAPI** | Postman, Insomnia | APIs y peticiones HTTP |
| **LixDB** | DBeaver, DataGrip, pgAdmin | Bases de datos |
| **LixGit** | GitHub Desktop, GitKraken | Cliente Git visual |
| **LixContainers** | Docker Desktop, Podman Desktop | Contenedores |
| **LixAI** | Ollama GUI, OpenAI tools, LM Studio | IA local/cloud, modelos y agentes |
| **LixML** | Weka, Orange, herramientas ML | Machine learning visual |

---

# 9. GIS, topografía, territorio e hidráulica

| App Lix | Sustituye / agrupa | Módulos internos |
|---|---|---|
| **LixGIS** | ArcGIS, QGIS, Global Mapper | GIS, mapas, capas y análisis espacial |
| **LixTerrain** | Herramientas DEM, topografía, Global Mapper | Terreno, curvas de nivel y nubes de puntos |
| **LixHydro** | HEC-RAS, EPANET, SWMM | Hidráulica, redes de agua y drenaje |
| **LixMaps** | QGIS ligero, mapas técnicos | Mapas y visualización territorial |
| **LixInfra** | InfraWorks, Civil 3D infraestructuras | Infraestructura 3D |

---

# 10. Química, biología y laboratorio científico

| App Lix | Sustituye / agrupa | Módulos internos |
|---|---|---|
| **LixChem** | ChemDraw, Avogadro, Gaussian UI | Química, moléculas y estructuras |
| **LixBio** | PyMOL, ImageJ, Fiji | Biología, imagen científica y visualización molecular |
| **LixPrism** | GraphPad Prism, Origin bioestadística | Bioestadística y gráficas científicas |
| **LixQuantum** | Gaussian, GaussView, química computacional | Química cuántica y simulación molecular |
| **LixMolecule** | Avogadro, PyMOL | Modelado molecular |

---

# 11. Apps principales recomendadas para empezar

Esta es la lista inicial más equilibrada para crear un ecosistema potente sin hacer demasiadas apps desde el principio.

## Sistema

| Prioridad | App |
|---:|---|
| 1 | **LixFiles** |
| 2 | **LixSettings** |
| 3 | **LixControl Center** |
| 4 | **LixBar** |
| 5 | **LixGreeter** |
| 6 | **LixLock** |
| 7 | **LixDock** |
| 8 | **LixStore** |
| 9 | **LixUpdater** |
| 10 | **LixDrop** |

## Productividad

| Prioridad | App |
|---:|---|
| 11 | **LixOffice** |
| 12 | **LixPages** |
| 13 | **LixNumbers** |
| 14 | **LixSlides** |
| 15 | **LixPDF** |
| 16 | **LixNotes** |
| 17 | **LixCite** |
| 18 | **LixDiagram** |

## Creatividad

| Prioridad | App |
|---:|---|
| 19 | **LixStudio** |
| 20 | **LixPhoto** |
| 21 | **LixDesign** |
| 22 | **LixAudio** |
| 23 | **Lix3D** |
| 24 | **LixCapture** |

## Ingeniería y ciencia

| Prioridad | App |
|---:|---|
| 25 | **LixCAD** |
| 26 | **LixModel** |
| 27 | **LixMath** |
| 28 | **LixSim** |
| 29 | **LixPlot** |
| 30 | **LixCFD** |
| 31 | **LixStructures** |
| 32 | **LixAirfoil** |
| 33 | **LixUnits** |
| 34 | **LixCAE** |
| 35 | **LixFEA** |

## Desarrollo y datos

| Prioridad | App |
|---:|---|
| 36 | **LixCode** |
| 37 | **LixDevkit** |
| 38 | **LixNotebook** |
| 39 | **LixData** |
| 40 | **LixAI** |

---

# 12. Núcleo mínimo realista de Nodalix OS

Si el objetivo es tener una primera versión coherente y usable, el núcleo mínimo debería ser:

| Categoría | Apps |
|---|---|
| Sistema | **LixFiles**, **LixSettings**, **LixControl Center**, **LixBar**, **LixGreeter**, **LixLock** |
| Productividad | **LixPages**, **LixNumbers**, **LixSlides**, **LixPDF**, **LixNotes** |
| Creatividad | **LixPhoto**, **LixStudio**, **LixDesign** |
| Ingeniería | **LixCAD**, **LixMath**, **LixSim**, **LixPlot**, **LixUnits** |
| Desarrollo | **LixCode**, **LixDevkit** |
| Utilidades | **LixDrop**, **LixCapture**, **LixStore**, **LixUpdater** |

Total aproximado inicial: **24 apps**.

---

# 13. Filosofía final

Nodalix OS no debería intentar parecer una colección de clones.

La filosofía debería ser:

> Una suite nativa, coherente y elegante para Linux, enfocada en productividad, creatividad, ingeniería y ciencia.

Cada app Lix debe:
- tener identidad visual propia,
- compartir diseño común,
- integrarse con Nodalix OS,
- evitar depender de interfaces antiguas,
- priorizar flujos reales de trabajo,
- ser honesta con sus limitaciones,
- y crecer por módulos internos en lugar de multiplicar apps innecesarias.

---
