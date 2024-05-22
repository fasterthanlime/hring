//! Macros to help generate code for all suites/groups/tests of the httpwg crate

// This file is automatically @generated by httpwg-gen
// It is not intended for manual editing

/// This generates a module tree with some #[test] functions.
/// The `$body` argument is pasted inside those unit test, and
/// in that scope, `test` is the `httpwg` function you can use
/// to run the test (that takes a `mut conn: Conn<IO>`)
#[macro_export]
macro_rules! tests {
    ($body: tt) => {
        /// RFC 9113 describes an optimized expression of the
        /// semantics of the Hypertext Transfer Protocol (HTTP), referred to as
        /// HTTP version 2 (HTTP/2).
        ///
        /// HTTP/2 enables a more efficient use of network resources and a reduced
        /// latency by introducing field compression and allowing multiple concurrent
        /// exchanges on the same connection.
        ///
        /// This document obsoletes RFCs 7540 and 8740.
        ///
        /// cf. <https://httpwg.org/specs/rfc9113.html>
        #[cfg(test)]
        mod rfc9113 {
            use httpwg::rfc9113 as __suite;

            /// Section 3: Starting HTTP/2
            mod _3_starting_http2 {
                use httpwg::rfc9113 as __suite;

                /// The server connection preface consists of a potentially empty
                /// SETTINGS frame (Section 6.5) that MUST be the first frame
                /// the server sends in the HTTP/2 connection.
                #[test]
                fn sends_client_connection_preface() {
                    use __suite::sends_client_connection_preface as test;
                    $body
                }

                /// Clients and servers MUST treat an invalid connection preface as
                /// a connection error (Section 5.4.1) of type PROTOCOL_ERROR.
                #[test]
                fn sends_invalid_connection_preface() {
                    use __suite::sends_invalid_connection_preface as test;
                    $body
                }
            }

            /// Section 4.2: Frame Size
            mod _4_2_frame_size {
                use httpwg::rfc9113 as __suite;

                /// An endpoint MUST send an error code of FRAME_SIZE_ERROR if a frame
                /// exceeds the size defined in SETTINGS_MAX_FRAME_SIZE, exceeds any
                /// limit defined for the frame type, or is too small to contain mandatory frame data
                #[test]
                fn frame_exceeding_max_size() {
                    use __suite::frame_exceeding_max_size as test;
                    $body
                }
            }
        }
    };
}
