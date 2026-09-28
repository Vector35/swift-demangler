//! Tests that malformed input fails cleanly, deep or long input doesn't
//! overflow the stack, and real symbols keep demangling.
//!
//! Everything runs on a 512 KiB thread, or 1 MiB on Windows, matching the
//! smallest default thread stacks on the supported platforms.

use swift_demangler::Symbol;
use swift_demangler::raw::{Context, demangle};

#[cfg(not(windows))]
const SMALL_STACK: usize = 512 * 1024;
#[cfg(windows)]
const SMALL_STACK: usize = 1024 * 1024;

fn on_small_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(SMALL_STACK)
        .spawn(f)
        .unwrap()
        .join()
        .unwrap()
}

/// Parse, print and drop `mangled` through every public entry point.
fn exercise(mangled: &str) -> Option<String> {
    let ctx = Context::new();
    let demangled = demangle(mangled);
    let symbol = Symbol::parse(&ctx, mangled);
    assert_eq!(
        demangled.is_some(),
        symbol.is_some(),
        "demangle and Symbol::parse disagree for {mangled}"
    );
    if let Some(symbol) = &symbol {
        symbol.display();
    }
    demangled
}

#[test]
fn keeps_working() {
    let long_identifier = format!("f{}", "2147483647x".repeat(17));
    let cases = [
        (
            "$s4main5helloSSyYaKF".to_string(),
            "main.hello() async throws -> Swift.String".to_string(),
        ),
        (
            "$s4main1fyySizF".into(),
            "main.f(inout Swift.Int) -> ()".into(),
        ),
        ("$s4main1fyyxlF".into(), "main.f<A>(A) -> ()".into()),
        ("$s4main1fyyq_r0_lF".into(), "main.f<A, B>(B) -> ()".into()),
        (
            "$s4main11f2147483647yyF".into(),
            "main.f2147483647() -> ()".into(),
        ),
        (
            "$s11a214748364711b2147483647yyF".into(),
            "a2147483647.b2147483647() -> ()".into(),
        ),
        (
            format!("$s4main{}{long_identifier}yyF", long_identifier.len()),
            format!("main.{long_identifier}() -> ()"),
        ),
    ];
    on_small_stack(move || {
        for (mangled, expected) in cases {
            assert_eq!(
                exercise(&mangled).as_deref(),
                Some(expected.as_str()),
                "{mangled}"
            );
        }
    });
}

/// `hello` wrapped in `count` generic specializations, which the parser
/// represents as `count + 1` siblings under `Global`.
fn stacked_specializations(count: usize) -> String {
    format!("$s4main5helloSSyYaKF{}", "yTg5".repeat(count))
}

fn check_specialization_chain(count: usize) {
    on_small_stack(move || {
        let mangled = stacked_specializations(count);
        let expected = format!(
            "{}main.hello() async throws -> Swift.String",
            "generic specialization <> of ".repeat(count)
        );
        assert_eq!(exercise(&mangled), Some(expected));

        let ctx = Context::new();
        let symbol = Symbol::parse(&ctx, &mangled).unwrap();
        let mut layers = 0;
        let mut inner = &symbol;
        while let Symbol::Specialization(s) = inner {
            layers += 1;
            inner = &s.inner;
        }
        assert_eq!(layers, count);
        assert!(inner.is_function());
    });
}

#[test]
fn specialization_chain_3() {
    check_specialization_chain(3);
}

#[test]
fn specialization_chain_255() {
    check_specialization_chain(255);
}

#[test]
fn specialization_chain_5000() {
    check_specialization_chain(5000);
}

#[test]
fn specialization_chain_30000() {
    check_specialization_chain(30000);
}

#[test]
fn real_symbols() {
    let cases = [
        // SwiftPM (Xcode 26.5), depth 42.
        r"$s9Workspace42ADPSwiftPackageCollectionCertificatePolicyV8validate9certChain14validationTimeySay4X5090E0VG_10Foundation4DateVtYaKFAG0F7BuilderV16buildFinalResultyQrxAG08VerifierF0RzlFZQOy_AO0P12PartialBlock11accumulated4nextQrx_q_tAgQRzAgQR_r0_lFZQOy_AorsTQrx_q_tAgQRzAgQR_r0_lFZQOy_AorsTQrx_q_tAgQRzAgQR_r0_lFZQOy_AorsTQrx_q_tAgQRzAgQR_r0_lFZQOy_AorsTQrx_q_tAgQRzAgQR_r0_lFZQOy_AA01_bceF0VAA015_ADPCertificateF0VQo_AA012_SubjectNameF0VQo_AA012_CodeSigningF0VQo_AG07RFC5280F0VQo_AA013_OCSPVerifierF0VQo_Qo_yXEfU_TA",
        // The deepest in SourceEditor (Xcode 26.5), depth 106.
        r"$s12SourceEditor18CodeCompletionListV4bodyQrvg7SwiftUI4ViewPAEE7onHover7performQrySbc_tFQOyAgEE0J6Change2of7initial_Qrqd___Sbyqd___qd__tctSQRd__lFQOyAgEE0J7Receive_AIQrqd___y6OutputQyd__ct7Combine9PublisherRd__s5NeverO7FailureRtd__lFQOyAgEE0J6AppearAIQryycSg_tFQOyAgEE7clipped11antialiasedQrSb_tFQOyAgEE5frame5width6height9alignmentQr12CoreGraphics7CGFloatVSg_A5_AE9AlignmentVtFQOyAgEE18scrollClipDisabledyQrSbFQOyAgEE14contentMargins__3forQrAE4EdgeO3SetV_AE10EdgeInsetsVAE22ContentMarginPlacementVtFQOyAE06ScrollI0VyAgEE31speechAlwaysIncludesPunctuationyQrSbFQOyAE15ModifiedContentVyAgEE20accessibilityFocusedyQrAE23AccessibilityFocusStateV7BindingVySb_GFQOyAE10LazyVStackVyAE7ForEachVySaySiGSiAgEE6zIndexyQrSdFQOyA23_yAgEE19simultaneousGesture_9includingQrqd___AE11GestureMaskVtAE7GestureRd__lFQOyAgEE7gesture_A37_Qrqd___A39_tAEA40_Rd__lFQOyAgEE7paddingyQrA16_FQOyAgEE12contentShape_6eoFillQrqd___SbtAE5ShapeRd__lFQOyAgEEAvIQrAW_tFQOyAA0cD3RowV_Qo__AE9RectangleVQo__Qo__AE13_EndedGestureVyAE10TapGestureVGQo__A57_Qo_AE31AccessibilityAttachmentModifierVG_Qo_SgGG_Qo_A61_G_Qo_G_Qo__Qo__Qo__Qo__Qo__AP9PublishedVAQVySiSg_GQo__A79_Qo__Qo_AE06ScrollI5ProxyVcfU0_A69_yXEfU_A65_yXEfU_A64_SicfU_yycfU_TATm",
        // The deepest real symbol, depth 143.
        r"$s7SwiftUI6HStackVyAA9TupleViewVyAA15ModifiedContentVyAA6VStackVyAA0E0PAAE9formStyleyQrqd__AA04FormJ0Rd__lFQOyAA0K0VyAEyAA7SectionVyAA4TextVAEyAA07LabeledG0VyA2SG_A4VtGAA05EmptyE0VG_AQyAsIyAEyAA0M5FieldVyASG_AA7DividerVA1_A3_A1_A3_ACyAEyAA6SpacerV_AGyAA6ButtonVyASGAA32_EnvironmentKeyTransformModifierVySbGGtGGtGGACyAEyA5__A8_tGGGtGG_AA07GroupedkJ0VQo_GAA12_FrameLayoutVG_AkAE4task4name8priority4file4line_QrSSSg_ScPSSSiyyYaYAcntFQOyAGyAA012SubscriptionE0Vy7Combine12AnyPublisherVyyts5NeverOGAGyAIyAA012_ConditionalG0VyAkAE19simultaneousGesture_9includingQrqd___AA11GestureMaskVtAA7GestureRd__lFQOyAkAE12onTapGesture5count7performQrSi_yyctFQOyAK08_MapKit_aB0E17onMapCameraChange9frequency_QrA53_24MapCameraUpdateFrequencyV_yA53_22MapCameraUpdateContextVctFQOyAKA53_E03mapJ0yQrA53_03MapJ0VFQOyAKA53_E11mapControlsyQrqd__yXEAaJRd__lFQOyA53_3MapVyA53_03MapgE0VyA41_A53_0d3MapG0VyAA7ForEachVySnySiGSiA53_03MapG0PA53_E6stroke_5styleQrqd___AA06StrokeJ0VtAA05ShapeJ0Rd__lFQOyA53_11MapPolylineV_AA5ColorVQo_G_A71_ySay16CarPlaySimulator10RoutePointVG10Foundation4UUIDVA53_10AnnotationVyAsGyAkAE06buttonJ0yQrqd__AA09PrimitivesJ0Rd__lFQOyA7_yAGyAGyAA06_ShapeE0VyAA6CircleVA83_GA27_GAA08_OverlayW0VyAA011StrokeShapeE0VyA100_A83_AYGGGG_AA05PlainsJ0VQo_A104_yAA5GroupVyAGyAGyAGyAIyAEyAGyAGyAGyAGyAIyAEyAS_AIyAEyACyAEyAS_A5_AStGG_A117_A117_SgtGGtGGAA08_PaddingZ0VGA124_GA27_GAA011_BackgroundW0VyAGyAGyA98_yAA16RoundedRectangleVAA8MaterialVGAA13_ShadowEffectVGA104_yA106_yA131_AA017HierarchicalShapeJ0VAYGGGGG_AGyAGyAGyA98_yAA4PathVA133_GA27_GA136_GAA13_OffsetEffectVGtGGAA013_TraitWritingW0VyAA011ZIndexTraitU0VGGA151_GAA017_AllowsHitTestingW0VGSgGGGGGA71_yA89_A92_A94_yAsGyAkAEA95_yQrqd__AAA96_Rd__lFQOyA7_yAGyAGyAGyAA5ImageVAA01_tu7WritingW0VyA83_SgGGA174_yAA4FontVSgGGA129_yA102_GGG_A112_Qo_A167_GGGA189_tGGG_A53_14MapZoomStepperVQo__Qo__Qo__Qo__AA13_EndedGestureVyAA10TapGestureVGQo_AA6ZStackVyAEyAGyAsA05_FlexyZ0VG_AIyAEyAGyAGyAGyAGyACyAEyAGyAGyA177_A124_GA124_G_AGyAGyAGyASA124_GA124_GA208_GtGGA208_GAA011_BackgroundjW0VyA83_GGAA11_ClipEffectVyA131_GGA124_G_A5_tGGtGGGGA208_GGAA017_AppearanceActionW0VG_Qo_tGGACyxGAajAWL",
        // The real symbol that needs the most stack, from ParsingInternal in the
        // macOS 26.5 SDK.
        r"$s15ParsingInternal13ParserBuilderV16buildFinalResultyQrq31_7ContextQy31_Rs30_AA010ContextfulC0R31_x_q_t_q0_t_q1_t_q2_t_q3_t_q4_t_q5_t_q6_t_q7_t_q8_t_q9_t_q10_t_q11_t_q12_t_q13_t_q14_t_q15_t_q16_t_q17_t_q18_t_q19_t_q20_t_q21_t_q22_t_q23_t_q24_t_q25_t_q26_t_q27_t_q28_t_q29_t6OutputRt31_r32_lFZ",
    ];
    on_small_stack(move || {
        for mangled in cases {
            let demangled = exercise(mangled).unwrap_or_else(|| panic!("{mangled} failed"));
            assert!(!demangled.contains("<<too complex>>"), "{mangled}");
        }
    });
}

/// Inputs that must fail.
const MUST_FAIL: &[&str] = &[
    "$sSiTQ2147483647_",
    "$s999999999999999999999999",
    "$s4main5helloSSyYaKFTQ2147483647_",
    "$s4main1fyyq2147483646_lF",
    "$s4main1fyyqd2147483646__lF",
];

#[test]
fn fails_cleanly() {
    on_small_stack(|| {
        for mangled in MUST_FAIL {
            assert_eq!(exercise(mangled), None, "{mangled}");
        }
    });
}
