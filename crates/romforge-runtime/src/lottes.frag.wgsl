// Generated from the pinned upstream CRT-Lottes by tools/compile-shader.
// Timothy Lottes: public domain; see assets/shaders/crt/shaders/crt-lottes.slang.
diagnostic(off, derivative_uniformity);
struct Push {
    hardScan: f32,
    hardPix: f32,
    warpX: f32,
    warpY: f32,
    maskDark: f32,
    maskLight: f32,
    scaleInLinearGamma: f32,
    shadowMask: f32,
    brightBoost: f32,
    hardBloomScan: f32,
    hardBloomPix: f32,
    bloomAmount: f32,
    shape: f32,
}

struct UBO {
    MVP: mat4x4<f32>,
    OutputSize: vec4<f32>,
    OriginalSize: vec4<f32>,
    SourceSize: vec4<f32>,
}

@group(0) @binding(3) 
var<uniform> params: Push;
@group(0) @binding(0) 
var<uniform> global: UBO;
@group(0) @binding(2) 
var Source: texture_2d<f32>;
var<private> vTexCoord_1: vec2<f32>;
var<private> FragColor: vec4<f32>;
@group(1) @binding(2) 
var _Source_sampler: sampler;

fn ToSrgb1_u0028_f1_u003b(c: ptr<function, f32>) -> f32 {
    var local: f32;

    let _e48 = params.scaleInLinearGamma;
    if (_e48 == 0f) {
        let _e50 = (*c);
        return _e50;
    }
    let _e51 = (*c);
    if (_e51 < 0.0031308f) {
        let _e53 = (*c);
        local = (_e53 * 12.92f);
    } else {
        let _e55 = (*c);
        local = ((1.055f * pow(_e55, 0.41666f)) - 0.055f);
    }
    let _e59 = local;
    return _e59;
}

fn ToSrgb_u0028_vf3_u003b(c_1: ptr<function, vec3<f32>>) -> vec3<f32> {
    var param: f32;
    var param_1: f32;
    var param_2: f32;

    let _e50 = params.scaleInLinearGamma;
    if (_e50 == 0f) {
        let _e52 = (*c_1);
        return _e52;
    }
    let _e54 = (*c_1)[0u];
    param = _e54;
    let _e55 = ToSrgb1_u0028_f1_u003b((&param));
    let _e57 = (*c_1)[1u];
    param_1 = _e57;
    let _e58 = ToSrgb1_u0028_f1_u003b((&param_1));
    let _e60 = (*c_1)[2u];
    param_2 = _e60;
    let _e61 = ToSrgb1_u0028_f1_u003b((&param_2));
    return vec3<f32>(_e55, _e58, _e61);
}

fn Mask_u0028_vf2_u003b(pos: ptr<function, vec2<f32>>) -> vec3<f32> {
    var mask: vec3<f32>;
    var line: f32;
    var odd: f32;

    let _e50 = params.maskDark;
    let _e52 = params.maskDark;
    let _e54 = params.maskDark;
    mask = vec3<f32>(_e50, _e52, _e54);
    let _e57 = params.shadowMask;
    if (_e57 == 1f) {
        let _e60 = params.maskLight;
        line = _e60;
        odd = 0f;
        let _e62 = (*pos)[0u];
        if (fract((_e62 * 0.16666667f)) < 0.5f) {
            odd = 1f;
        }
        let _e67 = (*pos)[1u];
        let _e68 = odd;
        if (fract(((_e67 + _e68) * 0.5f)) < 0.5f) {
            let _e74 = params.maskDark;
            line = _e74;
        }
        let _e76 = (*pos)[0u];
        (*pos)[0u] = fract((_e76 * 0.33333334f));
        let _e81 = (*pos)[0u];
        if (_e81 < 0.333f) {
            let _e84 = params.maskLight;
            mask[0u] = _e84;
        } else {
            let _e87 = (*pos)[0u];
            if (_e87 < 0.666f) {
                let _e90 = params.maskLight;
                mask[1u] = _e90;
            } else {
                let _e93 = params.maskLight;
                mask[2u] = _e93;
            }
        }
        let _e95 = line;
        let _e96 = mask;
        mask = (_e96 * _e95);
    } else {
        let _e99 = params.shadowMask;
        if (_e99 == 2f) {
            let _e102 = (*pos)[0u];
            (*pos)[0u] = fract((_e102 * 0.33333334f));
            let _e107 = (*pos)[0u];
            if (_e107 < 0.333f) {
                let _e110 = params.maskLight;
                mask[0u] = _e110;
            } else {
                let _e113 = (*pos)[0u];
                if (_e113 < 0.666f) {
                    let _e116 = params.maskLight;
                    mask[1u] = _e116;
                } else {
                    let _e119 = params.maskLight;
                    mask[2u] = _e119;
                }
            }
        } else {
            let _e122 = params.shadowMask;
            if (_e122 == 3f) {
                let _e125 = (*pos)[1u];
                let _e128 = (*pos)[0u];
                (*pos)[0u] = (_e128 + (_e125 * 3f));
                let _e132 = (*pos)[0u];
                (*pos)[0u] = fract((_e132 * 0.16666667f));
                let _e137 = (*pos)[0u];
                if (_e137 < 0.333f) {
                    let _e140 = params.maskLight;
                    mask[0u] = _e140;
                } else {
                    let _e143 = (*pos)[0u];
                    if (_e143 < 0.666f) {
                        let _e146 = params.maskLight;
                        mask[1u] = _e146;
                    } else {
                        let _e149 = params.maskLight;
                        mask[2u] = _e149;
                    }
                }
            } else {
                let _e152 = params.shadowMask;
                if (_e152 == 4f) {
                    let _e154 = (*pos);
                    (*pos) = floor((_e154 * vec2<f32>(1f, 0.5f)));
                    let _e158 = (*pos)[1u];
                    let _e161 = (*pos)[0u];
                    (*pos)[0u] = (_e161 + (_e158 * 3f));
                    let _e165 = (*pos)[0u];
                    (*pos)[0u] = fract((_e165 * 0.16666667f));
                    let _e170 = (*pos)[0u];
                    if (_e170 < 0.333f) {
                        let _e173 = params.maskLight;
                        mask[0u] = _e173;
                    } else {
                        let _e176 = (*pos)[0u];
                        if (_e176 < 0.666f) {
                            let _e179 = params.maskLight;
                            mask[1u] = _e179;
                        } else {
                            let _e182 = params.maskLight;
                            mask[2u] = _e182;
                        }
                    }
                }
            }
        }
    }
    let _e184 = mask;
    return _e184;
}

fn Gaus_u0028_f1_u003b_f1_u003b(pos_1: ptr<function, f32>, scale: ptr<function, f32>) -> f32 {
    let _e47 = (*scale);
    let _e48 = (*pos_1);
    let _e51 = params.shape;
    return exp2((_e47 * pow(abs(_e48), _e51)));
}

fn Dist_u0028_vf2_u003b(pos_2: ptr<function, vec2<f32>>) -> vec2<f32> {
    let _e46 = (*pos_2);
    let _e48 = global.SourceSize;
    (*pos_2) = (_e46 * _e48.xy);
    let _e51 = (*pos_2);
    let _e52 = (*pos_2);
    return -(((_e51 - floor(_e52)) - vec2<f32>(0.5f, 0.5f)));
}

fn BloomScan_u0028_vf2_u003b_f1_u003b(pos_3: ptr<function, vec2<f32>>, off: ptr<function, f32>) -> f32 {
    var dst: f32;
    var param_3: vec2<f32>;
    var param_4: f32;
    var param_5: f32;

    let _e51 = (*pos_3);
    param_3 = _e51;
    let _e52 = Dist_u0028_vf2_u003b((&param_3));
    dst = _e52.y;
    let _e54 = dst;
    let _e55 = (*off);
    param_4 = (_e54 + _e55);
    let _e58 = params.hardBloomScan;
    param_5 = _e58;
    let _e59 = Gaus_u0028_f1_u003b_f1_u003b((&param_4), (&param_5));
    return _e59;
}

fn ToLinear1_u0028_f1_u003b(c_2: ptr<function, f32>) -> f32 {
    var local_1: f32;

    let _e48 = params.scaleInLinearGamma;
    if (_e48 == 0f) {
        let _e50 = (*c_2);
        return _e50;
    }
    let _e51 = (*c_2);
    if (_e51 <= 0.04045f) {
        let _e53 = (*c_2);
        local_1 = (_e53 / 12.92f);
    } else {
        let _e55 = (*c_2);
        local_1 = pow(((_e55 + 0.055f) / 1.055f), 2.4f);
    }
    let _e59 = local_1;
    return _e59;
}

fn ToLinear_u0028_vf3_u003b(c_3: ptr<function, vec3<f32>>) -> vec3<f32> {
    var param_6: f32;
    var param_7: f32;
    var param_8: f32;

    let _e50 = params.scaleInLinearGamma;
    if (_e50 == 0f) {
        let _e52 = (*c_3);
        return _e52;
    }
    let _e54 = (*c_3)[0u];
    param_6 = _e54;
    let _e55 = ToLinear1_u0028_f1_u003b((&param_6));
    let _e57 = (*c_3)[1u];
    param_7 = _e57;
    let _e58 = ToLinear1_u0028_f1_u003b((&param_7));
    let _e60 = (*c_3)[2u];
    param_8 = _e60;
    let _e61 = ToLinear1_u0028_f1_u003b((&param_8));
    return vec3<f32>(_e55, _e58, _e61);
}

fn Fetch_u0028_vf2_u003b_vf2_u003b(pos_4: ptr<function, vec2<f32>>, off_1: ptr<function, vec2<f32>>) -> vec3<f32> {
    var param_9: vec3<f32>;

    let _e48 = (*pos_4);
    let _e50 = global.SourceSize;
    let _e53 = (*off_1);
    let _e58 = global.SourceSize;
    (*pos_4) = ((floor(((_e48 * _e50.xy) + _e53)) + vec2<f32>(0.5f, 0.5f)) / _e58.xy);
    let _e62 = params.brightBoost;
    let _e63 = (*pos_4);
    let _e64 = textureSample(Source, _Source_sampler, _e63);
    param_9 = (_e64.xyz * _e62);
    let _e67 = ToLinear_u0028_vf3_u003b((&param_9));
    return _e67;
}

fn Horz7_u0028_vf2_u003b_f1_u003b(pos_5: ptr<function, vec2<f32>>, off_2: ptr<function, f32>) -> vec3<f32> {
    var a: vec3<f32>;
    var param_10: vec2<f32>;
    var param_11: vec2<f32>;
    var b: vec3<f32>;
    var param_12: vec2<f32>;
    var param_13: vec2<f32>;
    var c_4: vec3<f32>;
    var param_14: vec2<f32>;
    var param_15: vec2<f32>;
    var d: vec3<f32>;
    var param_16: vec2<f32>;
    var param_17: vec2<f32>;
    var e: vec3<f32>;
    var param_18: vec2<f32>;
    var param_19: vec2<f32>;
    var f: vec3<f32>;
    var param_20: vec2<f32>;
    var param_21: vec2<f32>;
    var g: vec3<f32>;
    var param_22: vec2<f32>;
    var param_23: vec2<f32>;
    var dst_1: f32;
    var param_24: vec2<f32>;
    var scale_1: f32;
    var wa: f32;
    var param_25: f32;
    var param_26: f32;
    var wb: f32;
    var param_27: f32;
    var param_28: f32;
    var wc: f32;
    var param_29: f32;
    var param_30: f32;
    var wd: f32;
    var param_31: f32;
    var param_32: f32;
    var we: f32;
    var param_33: f32;
    var param_34: f32;
    var wf: f32;
    var param_35: f32;
    var param_36: f32;
    var wg: f32;
    var param_37: f32;
    var param_38: f32;

    let _e92 = (*off_2);
    let _e94 = (*pos_5);
    param_10 = _e94;
    param_11 = vec2<f32>(-3f, _e92);
    let _e95 = Fetch_u0028_vf2_u003b_vf2_u003b((&param_10), (&param_11));
    a = _e95;
    let _e96 = (*off_2);
    let _e98 = (*pos_5);
    param_12 = _e98;
    param_13 = vec2<f32>(-2f, _e96);
    let _e99 = Fetch_u0028_vf2_u003b_vf2_u003b((&param_12), (&param_13));
    b = _e99;
    let _e100 = (*off_2);
    let _e102 = (*pos_5);
    param_14 = _e102;
    param_15 = vec2<f32>(-1f, _e100);
    let _e103 = Fetch_u0028_vf2_u003b_vf2_u003b((&param_14), (&param_15));
    c_4 = _e103;
    let _e104 = (*off_2);
    let _e106 = (*pos_5);
    param_16 = _e106;
    param_17 = vec2<f32>(0f, _e104);
    let _e107 = Fetch_u0028_vf2_u003b_vf2_u003b((&param_16), (&param_17));
    d = _e107;
    let _e108 = (*off_2);
    let _e110 = (*pos_5);
    param_18 = _e110;
    param_19 = vec2<f32>(1f, _e108);
    let _e111 = Fetch_u0028_vf2_u003b_vf2_u003b((&param_18), (&param_19));
    e = _e111;
    let _e112 = (*off_2);
    let _e114 = (*pos_5);
    param_20 = _e114;
    param_21 = vec2<f32>(2f, _e112);
    let _e115 = Fetch_u0028_vf2_u003b_vf2_u003b((&param_20), (&param_21));
    f = _e115;
    let _e116 = (*off_2);
    let _e118 = (*pos_5);
    param_22 = _e118;
    param_23 = vec2<f32>(3f, _e116);
    let _e119 = Fetch_u0028_vf2_u003b_vf2_u003b((&param_22), (&param_23));
    g = _e119;
    let _e120 = (*pos_5);
    param_24 = _e120;
    let _e121 = Dist_u0028_vf2_u003b((&param_24));
    dst_1 = _e121.x;
    let _e124 = params.hardBloomPix;
    scale_1 = _e124;
    let _e125 = dst_1;
    param_25 = (_e125 - 3f);
    let _e127 = scale_1;
    param_26 = _e127;
    let _e128 = Gaus_u0028_f1_u003b_f1_u003b((&param_25), (&param_26));
    wa = _e128;
    let _e129 = dst_1;
    param_27 = (_e129 - 2f);
    let _e131 = scale_1;
    param_28 = _e131;
    let _e132 = Gaus_u0028_f1_u003b_f1_u003b((&param_27), (&param_28));
    wb = _e132;
    let _e133 = dst_1;
    param_29 = (_e133 - 1f);
    let _e135 = scale_1;
    param_30 = _e135;
    let _e136 = Gaus_u0028_f1_u003b_f1_u003b((&param_29), (&param_30));
    wc = _e136;
    let _e137 = dst_1;
    param_31 = (_e137 + 0f);
    let _e139 = scale_1;
    param_32 = _e139;
    let _e140 = Gaus_u0028_f1_u003b_f1_u003b((&param_31), (&param_32));
    wd = _e140;
    let _e141 = dst_1;
    param_33 = (_e141 + 1f);
    let _e143 = scale_1;
    param_34 = _e143;
    let _e144 = Gaus_u0028_f1_u003b_f1_u003b((&param_33), (&param_34));
    we = _e144;
    let _e145 = dst_1;
    param_35 = (_e145 + 2f);
    let _e147 = scale_1;
    param_36 = _e147;
    let _e148 = Gaus_u0028_f1_u003b_f1_u003b((&param_35), (&param_36));
    wf = _e148;
    let _e149 = dst_1;
    param_37 = (_e149 + 3f);
    let _e151 = scale_1;
    param_38 = _e151;
    let _e152 = Gaus_u0028_f1_u003b_f1_u003b((&param_37), (&param_38));
    wg = _e152;
    let _e153 = a;
    let _e154 = wa;
    let _e156 = b;
    let _e157 = wb;
    let _e160 = c_4;
    let _e161 = wc;
    let _e164 = d;
    let _e165 = wd;
    let _e168 = e;
    let _e169 = we;
    let _e172 = f;
    let _e173 = wf;
    let _e176 = g;
    let _e177 = wg;
    let _e180 = wa;
    let _e181 = wb;
    let _e183 = wc;
    let _e185 = wd;
    let _e187 = we;
    let _e189 = wf;
    let _e191 = wg;
    return ((((((((_e153 * _e154) + (_e156 * _e157)) + (_e160 * _e161)) + (_e164 * _e165)) + (_e168 * _e169)) + (_e172 * _e173)) + (_e176 * _e177)) / vec3(((((((_e180 + _e181) + _e183) + _e185) + _e187) + _e189) + _e191)));
}

fn Horz5_u0028_vf2_u003b_f1_u003b(pos_6: ptr<function, vec2<f32>>, off_3: ptr<function, f32>) -> vec3<f32> {
    var a_1: vec3<f32>;
    var param_39: vec2<f32>;
    var param_40: vec2<f32>;
    var b_1: vec3<f32>;
    var param_41: vec2<f32>;
    var param_42: vec2<f32>;
    var c_5: vec3<f32>;
    var param_43: vec2<f32>;
    var param_44: vec2<f32>;
    var d_1: vec3<f32>;
    var param_45: vec2<f32>;
    var param_46: vec2<f32>;
    var e_1: vec3<f32>;
    var param_47: vec2<f32>;
    var param_48: vec2<f32>;
    var dst_2: f32;
    var param_49: vec2<f32>;
    var scale_2: f32;
    var wa_1: f32;
    var param_50: f32;
    var param_51: f32;
    var wb_1: f32;
    var param_52: f32;
    var param_53: f32;
    var wc_1: f32;
    var param_54: f32;
    var param_55: f32;
    var wd_1: f32;
    var param_56: f32;
    var param_57: f32;
    var we_1: f32;
    var param_58: f32;
    var param_59: f32;

    let _e80 = (*off_3);
    let _e82 = (*pos_6);
    param_39 = _e82;
    param_40 = vec2<f32>(-2f, _e80);
    let _e83 = Fetch_u0028_vf2_u003b_vf2_u003b((&param_39), (&param_40));
    a_1 = _e83;
    let _e84 = (*off_3);
    let _e86 = (*pos_6);
    param_41 = _e86;
    param_42 = vec2<f32>(-1f, _e84);
    let _e87 = Fetch_u0028_vf2_u003b_vf2_u003b((&param_41), (&param_42));
    b_1 = _e87;
    let _e88 = (*off_3);
    let _e90 = (*pos_6);
    param_43 = _e90;
    param_44 = vec2<f32>(0f, _e88);
    let _e91 = Fetch_u0028_vf2_u003b_vf2_u003b((&param_43), (&param_44));
    c_5 = _e91;
    let _e92 = (*off_3);
    let _e94 = (*pos_6);
    param_45 = _e94;
    param_46 = vec2<f32>(1f, _e92);
    let _e95 = Fetch_u0028_vf2_u003b_vf2_u003b((&param_45), (&param_46));
    d_1 = _e95;
    let _e96 = (*off_3);
    let _e98 = (*pos_6);
    param_47 = _e98;
    param_48 = vec2<f32>(2f, _e96);
    let _e99 = Fetch_u0028_vf2_u003b_vf2_u003b((&param_47), (&param_48));
    e_1 = _e99;
    let _e100 = (*pos_6);
    param_49 = _e100;
    let _e101 = Dist_u0028_vf2_u003b((&param_49));
    dst_2 = _e101.x;
    let _e104 = params.hardPix;
    scale_2 = _e104;
    let _e105 = dst_2;
    param_50 = (_e105 - 2f);
    let _e107 = scale_2;
    param_51 = _e107;
    let _e108 = Gaus_u0028_f1_u003b_f1_u003b((&param_50), (&param_51));
    wa_1 = _e108;
    let _e109 = dst_2;
    param_52 = (_e109 - 1f);
    let _e111 = scale_2;
    param_53 = _e111;
    let _e112 = Gaus_u0028_f1_u003b_f1_u003b((&param_52), (&param_53));
    wb_1 = _e112;
    let _e113 = dst_2;
    param_54 = (_e113 + 0f);
    let _e115 = scale_2;
    param_55 = _e115;
    let _e116 = Gaus_u0028_f1_u003b_f1_u003b((&param_54), (&param_55));
    wc_1 = _e116;
    let _e117 = dst_2;
    param_56 = (_e117 + 1f);
    let _e119 = scale_2;
    param_57 = _e119;
    let _e120 = Gaus_u0028_f1_u003b_f1_u003b((&param_56), (&param_57));
    wd_1 = _e120;
    let _e121 = dst_2;
    param_58 = (_e121 + 2f);
    let _e123 = scale_2;
    param_59 = _e123;
    let _e124 = Gaus_u0028_f1_u003b_f1_u003b((&param_58), (&param_59));
    we_1 = _e124;
    let _e125 = a_1;
    let _e126 = wa_1;
    let _e128 = b_1;
    let _e129 = wb_1;
    let _e132 = c_5;
    let _e133 = wc_1;
    let _e136 = d_1;
    let _e137 = wd_1;
    let _e140 = e_1;
    let _e141 = we_1;
    let _e144 = wa_1;
    let _e145 = wb_1;
    let _e147 = wc_1;
    let _e149 = wd_1;
    let _e151 = we_1;
    return ((((((_e125 * _e126) + (_e128 * _e129)) + (_e132 * _e133)) + (_e136 * _e137)) + (_e140 * _e141)) / vec3(((((_e144 + _e145) + _e147) + _e149) + _e151)));
}

fn Bloom_u0028_vf2_u003b(pos_7: ptr<function, vec2<f32>>) -> vec3<f32> {
    var a_2: vec3<f32>;
    var param_60: vec2<f32>;
    var param_61: f32;
    var b_2: vec3<f32>;
    var param_62: vec2<f32>;
    var param_63: f32;
    var c_6: vec3<f32>;
    var param_64: vec2<f32>;
    var param_65: f32;
    var d_2: vec3<f32>;
    var param_66: vec2<f32>;
    var param_67: f32;
    var e_2: vec3<f32>;
    var param_68: vec2<f32>;
    var param_69: f32;
    var wa_2: f32;
    var param_70: vec2<f32>;
    var param_71: f32;
    var wb_2: f32;
    var param_72: vec2<f32>;
    var param_73: f32;
    var wc_2: f32;
    var param_74: vec2<f32>;
    var param_75: f32;
    var wd_2: f32;
    var param_76: vec2<f32>;
    var param_77: f32;
    var we_2: f32;
    var param_78: vec2<f32>;
    var param_79: f32;

    let _e76 = (*pos_7);
    param_60 = _e76;
    param_61 = -2f;
    let _e77 = Horz5_u0028_vf2_u003b_f1_u003b((&param_60), (&param_61));
    a_2 = _e77;
    let _e78 = (*pos_7);
    param_62 = _e78;
    param_63 = -1f;
    let _e79 = Horz7_u0028_vf2_u003b_f1_u003b((&param_62), (&param_63));
    b_2 = _e79;
    let _e80 = (*pos_7);
    param_64 = _e80;
    param_65 = 0f;
    let _e81 = Horz7_u0028_vf2_u003b_f1_u003b((&param_64), (&param_65));
    c_6 = _e81;
    let _e82 = (*pos_7);
    param_66 = _e82;
    param_67 = 1f;
    let _e83 = Horz7_u0028_vf2_u003b_f1_u003b((&param_66), (&param_67));
    d_2 = _e83;
    let _e84 = (*pos_7);
    param_68 = _e84;
    param_69 = 2f;
    let _e85 = Horz5_u0028_vf2_u003b_f1_u003b((&param_68), (&param_69));
    e_2 = _e85;
    let _e86 = (*pos_7);
    param_70 = _e86;
    param_71 = -2f;
    let _e87 = BloomScan_u0028_vf2_u003b_f1_u003b((&param_70), (&param_71));
    wa_2 = _e87;
    let _e88 = (*pos_7);
    param_72 = _e88;
    param_73 = -1f;
    let _e89 = BloomScan_u0028_vf2_u003b_f1_u003b((&param_72), (&param_73));
    wb_2 = _e89;
    let _e90 = (*pos_7);
    param_74 = _e90;
    param_75 = 0f;
    let _e91 = BloomScan_u0028_vf2_u003b_f1_u003b((&param_74), (&param_75));
    wc_2 = _e91;
    let _e92 = (*pos_7);
    param_76 = _e92;
    param_77 = 1f;
    let _e93 = BloomScan_u0028_vf2_u003b_f1_u003b((&param_76), (&param_77));
    wd_2 = _e93;
    let _e94 = (*pos_7);
    param_78 = _e94;
    param_79 = 2f;
    let _e95 = BloomScan_u0028_vf2_u003b_f1_u003b((&param_78), (&param_79));
    we_2 = _e95;
    let _e96 = a_2;
    let _e97 = wa_2;
    let _e99 = b_2;
    let _e100 = wb_2;
    let _e103 = c_6;
    let _e104 = wc_2;
    let _e107 = d_2;
    let _e108 = wd_2;
    let _e111 = e_2;
    let _e112 = we_2;
    return (((((_e96 * _e97) + (_e99 * _e100)) + (_e103 * _e104)) + (_e107 * _e108)) + (_e111 * _e112));
}

fn Scan_u0028_vf2_u003b_f1_u003b(pos_8: ptr<function, vec2<f32>>, off_4: ptr<function, f32>) -> f32 {
    var dst_3: f32;
    var param_80: vec2<f32>;
    var param_81: f32;
    var param_82: f32;

    let _e51 = (*pos_8);
    param_80 = _e51;
    let _e52 = Dist_u0028_vf2_u003b((&param_80));
    dst_3 = _e52.y;
    let _e54 = dst_3;
    let _e55 = (*off_4);
    param_81 = (_e54 + _e55);
    let _e58 = params.hardScan;
    param_82 = _e58;
    let _e59 = Gaus_u0028_f1_u003b_f1_u003b((&param_81), (&param_82));
    return _e59;
}

fn Horz3_u0028_vf2_u003b_f1_u003b(pos_9: ptr<function, vec2<f32>>, off_5: ptr<function, f32>) -> vec3<f32> {
    var b_3: vec3<f32>;
    var param_83: vec2<f32>;
    var param_84: vec2<f32>;
    var c_7: vec3<f32>;
    var param_85: vec2<f32>;
    var param_86: vec2<f32>;
    var d_3: vec3<f32>;
    var param_87: vec2<f32>;
    var param_88: vec2<f32>;
    var dst_4: f32;
    var param_89: vec2<f32>;
    var scale_3: f32;
    var wb_3: f32;
    var param_90: f32;
    var param_91: f32;
    var wc_3: f32;
    var param_92: f32;
    var param_93: f32;
    var wd_3: f32;
    var param_94: f32;
    var param_95: f32;

    let _e68 = (*off_5);
    let _e70 = (*pos_9);
    param_83 = _e70;
    param_84 = vec2<f32>(-1f, _e68);
    let _e71 = Fetch_u0028_vf2_u003b_vf2_u003b((&param_83), (&param_84));
    b_3 = _e71;
    let _e72 = (*off_5);
    let _e74 = (*pos_9);
    param_85 = _e74;
    param_86 = vec2<f32>(0f, _e72);
    let _e75 = Fetch_u0028_vf2_u003b_vf2_u003b((&param_85), (&param_86));
    c_7 = _e75;
    let _e76 = (*off_5);
    let _e78 = (*pos_9);
    param_87 = _e78;
    param_88 = vec2<f32>(1f, _e76);
    let _e79 = Fetch_u0028_vf2_u003b_vf2_u003b((&param_87), (&param_88));
    d_3 = _e79;
    let _e80 = (*pos_9);
    param_89 = _e80;
    let _e81 = Dist_u0028_vf2_u003b((&param_89));
    dst_4 = _e81.x;
    let _e84 = params.hardPix;
    scale_3 = _e84;
    let _e85 = dst_4;
    param_90 = (_e85 - 1f);
    let _e87 = scale_3;
    param_91 = _e87;
    let _e88 = Gaus_u0028_f1_u003b_f1_u003b((&param_90), (&param_91));
    wb_3 = _e88;
    let _e89 = dst_4;
    param_92 = (_e89 + 0f);
    let _e91 = scale_3;
    param_93 = _e91;
    let _e92 = Gaus_u0028_f1_u003b_f1_u003b((&param_92), (&param_93));
    wc_3 = _e92;
    let _e93 = dst_4;
    param_94 = (_e93 + 1f);
    let _e95 = scale_3;
    param_95 = _e95;
    let _e96 = Gaus_u0028_f1_u003b_f1_u003b((&param_94), (&param_95));
    wd_3 = _e96;
    let _e97 = b_3;
    let _e98 = wb_3;
    let _e100 = c_7;
    let _e101 = wc_3;
    let _e104 = d_3;
    let _e105 = wd_3;
    let _e108 = wb_3;
    let _e109 = wc_3;
    let _e111 = wd_3;
    return ((((_e97 * _e98) + (_e100 * _e101)) + (_e104 * _e105)) / vec3(((_e108 + _e109) + _e111)));
}

fn Tri_u0028_vf2_u003b(pos_10: ptr<function, vec2<f32>>) -> vec3<f32> {
    var a_3: vec3<f32>;
    var param_96: vec2<f32>;
    var param_97: f32;
    var b_4: vec3<f32>;
    var param_98: vec2<f32>;
    var param_99: f32;
    var c_8: vec3<f32>;
    var param_100: vec2<f32>;
    var param_101: f32;
    var wa_3: f32;
    var param_102: vec2<f32>;
    var param_103: f32;
    var wb_4: f32;
    var param_104: vec2<f32>;
    var param_105: f32;
    var wc_4: f32;
    var param_106: vec2<f32>;
    var param_107: f32;

    let _e64 = (*pos_10);
    param_96 = _e64;
    param_97 = -1f;
    let _e65 = Horz3_u0028_vf2_u003b_f1_u003b((&param_96), (&param_97));
    a_3 = _e65;
    let _e66 = (*pos_10);
    param_98 = _e66;
    param_99 = 0f;
    let _e67 = Horz5_u0028_vf2_u003b_f1_u003b((&param_98), (&param_99));
    b_4 = _e67;
    let _e68 = (*pos_10);
    param_100 = _e68;
    param_101 = 1f;
    let _e69 = Horz3_u0028_vf2_u003b_f1_u003b((&param_100), (&param_101));
    c_8 = _e69;
    let _e70 = (*pos_10);
    param_102 = _e70;
    param_103 = -1f;
    let _e71 = Scan_u0028_vf2_u003b_f1_u003b((&param_102), (&param_103));
    wa_3 = _e71;
    let _e72 = (*pos_10);
    param_104 = _e72;
    param_105 = 0f;
    let _e73 = Scan_u0028_vf2_u003b_f1_u003b((&param_104), (&param_105));
    wb_4 = _e73;
    let _e74 = (*pos_10);
    param_106 = _e74;
    param_107 = 1f;
    let _e75 = Scan_u0028_vf2_u003b_f1_u003b((&param_106), (&param_107));
    wc_4 = _e75;
    let _e76 = a_3;
    let _e77 = wa_3;
    let _e79 = b_4;
    let _e80 = wb_4;
    let _e83 = c_8;
    let _e84 = wc_4;
    return (((_e76 * _e77) + (_e79 * _e80)) + (_e83 * _e84));
}

fn Warp_u0028_vf2_u003b(pos_11: ptr<function, vec2<f32>>) -> vec2<f32> {
    let _e46 = (*pos_11);
    (*pos_11) = ((_e46 * 2f) - vec2(1f));
    let _e51 = (*pos_11)[1u];
    let _e53 = (*pos_11)[1u];
    let _e56 = params.warpX;
    let _e60 = (*pos_11)[0u];
    let _e62 = (*pos_11)[0u];
    let _e65 = params.warpY;
    let _e69 = (*pos_11);
    (*pos_11) = (_e69 * vec2<f32>((1f + ((_e51 * _e53) * _e56)), (1f + ((_e60 * _e62) * _e65))));
    let _e71 = (*pos_11);
    return ((_e71 * 0.5f) + vec2(0.5f));
}

fn main_1() {
    var pos_12: vec2<f32>;
    var param_108: vec2<f32>;
    var outColor: vec3<f32>;
    var param_109: vec2<f32>;
    var param_110: vec2<f32>;
    var param_111: vec2<f32>;
    var param_112: vec3<f32>;

    let _e52 = vTexCoord_1;
    param_108 = _e52;
    let _e53 = Warp_u0028_vf2_u003b((&param_108));
    pos_12 = _e53;
    let _e54 = pos_12;
    param_109 = _e54;
    let _e55 = Tri_u0028_vf2_u003b((&param_109));
    outColor = _e55;
    let _e56 = pos_12;
    param_110 = _e56;
    let _e57 = Bloom_u0028_vf2_u003b((&param_110));
    let _e59 = params.bloomAmount;
    let _e61 = outColor;
    outColor = (_e61 + (_e57 * _e59));
    let _e64 = params.shadowMask;
    if (_e64 > 0f) {
        let _e66 = vTexCoord_1;
        let _e68 = global.OutputSize;
        param_111 = ((_e66 / _e68.zw) * 1.000001f);
        let _e72 = Mask_u0028_vf2_u003b((&param_111));
        let _e73 = outColor;
        outColor = (_e73 * _e72);
    }
    let _e75 = outColor;
    param_112 = _e75;
    let _e76 = ToSrgb_u0028_vf3_u003b((&param_112));
    FragColor = vec4<f32>(_e76.x, _e76.y, _e76.z, 1f);
    return;
}

@fragment 
fn main(@location(0) vTexCoord: vec2<f32>) -> @location(0) vec4<f32> {
    vTexCoord_1 = vTexCoord;
    main_1();
    let _e3 = FragColor;
    return _e3;
}
