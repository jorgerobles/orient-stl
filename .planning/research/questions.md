# Open Research Questions

## 1. ASCII STL support — ¿necesario?

Fechado: 2026-07-11
Contexto: Decisión de parser STL en Rust. STL binario es ~50 bytes/triángulo, ASCII puede ser 10× más grande y más lento de parsear. La mayoría de slicers y herramientas de modelado exportan binario por defecto.
Criterio: Si los archivos STL reales que recibirá la herramienta son siempre binarios (decisión del pipeline aguas arriba), no implementar ASCII. Si hay posibilidad de archivos ASCII de fuentes externas, implementar con una detección de cabecera (línea `solid` / bytes no válidos).
