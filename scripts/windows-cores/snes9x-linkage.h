// These globals are shared by the pinned core's C and C++ translation units.
// MSVC decorates C++ data names; declare C linkage before upstream headers so
// both languages use the same names without modifying the upstream checkout.
#ifdef __cplusplus
struct SSettings;
struct SLineMatrixData;
extern "C" {
extern SSettings Settings;
extern SLineMatrixData LineMatrixData[];
extern unsigned char OpenBus;
}
#endif
