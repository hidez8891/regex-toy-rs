use super::super::*;

#[cfg(test)]
mod basic_match {
    use super::*;

    #[test]
    fn match_char() {
        let src = "abc";
        let vm = Nfa::new(src).unwrap();

        assert_eq!(vm.is_match("abc"), true);
        assert_eq!(vm.is_match("ab"), false);
        assert_eq!(vm.is_match("abcd"), true);
        assert_eq!(vm.is_match("zabc"), true);
    }

    #[test]
    fn match_metachar() {
        let src = r"a\+c";
        let vm = Nfa::new(src).unwrap();

        assert_eq!(vm.is_match("a+c"), true);
        assert_eq!(vm.is_match("aac"), false);
        assert_eq!(vm.is_match("ac"), false);
        assert_eq!(vm.is_match("a+cz"), true);
        assert_eq!(vm.is_match("za+c"), true);
    }

    #[test]
    fn match_any() {
        {
            let src = "a.c";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.is_match("abc"), true);
            assert_eq!(vm.is_match("adc"), true);
            assert_eq!(vm.is_match("ac"), false);
            assert_eq!(vm.is_match("abbc"), false);
            assert_eq!(vm.is_match("zabc"), true);
            assert_eq!(vm.is_match("abcz"), true);
        }
        {
            let src = "a.";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.is_match("ab"), true);
            assert_eq!(vm.is_match("ad"), true);
            assert_eq!(vm.is_match("a"), false);
            assert_eq!(vm.is_match("abz"), true);
            assert_eq!(vm.is_match("zab"), true);
        }
    }

    #[test]
    fn match_sol() {
        {
            let src = "^abc";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.is_match("abc"), true);
            assert_eq!(vm.is_match("zabc"), false);
            assert_eq!(vm.is_match("abcz"), true);
        }
    }

    #[test]
    fn match_eol() {
        {
            let src = "abc$";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.is_match("abc"), true);
            assert_eq!(vm.is_match("zabc"), true);
            assert_eq!(vm.is_match("abcz"), false);
        }
    }
}

#[test]
fn capture_group() {
    {
        let src = "a(bc)d";
        let vm = Nfa::new(src).unwrap();

        assert_eq!(vm.is_match("abcd"), true);
        assert_eq!(vm.is_match("abc"), false);
        assert_eq!(vm.is_match("ad"), false);
        assert_eq!(vm.is_match("zabcd"), true);
        assert_eq!(vm.is_match("abcdz"), true);
    }
    {
        let src = "a(bc)";
        let vm = Nfa::new(src).unwrap();

        assert_eq!(vm.is_match("abc"), true);
        assert_eq!(vm.is_match("a"), false);
        assert_eq!(vm.is_match("zabc"), true);
        assert_eq!(vm.is_match("abcd"), true);
    }
    {
        let src = "a(bc(de)f)(gh)";
        let vm = Nfa::new(src).unwrap();

        assert_eq!(vm.is_match("abcdefgh"), true);
        assert_eq!(vm.is_match("abcdef"), false);
        assert_eq!(vm.is_match("abcgh"), false);
        assert_eq!(vm.is_match("agh"), false);
    }
}

#[test]
fn noncapture_group() {
    {
        let src = "a(?:bc)d";
        let vm = Nfa::new(src).unwrap();

        assert_eq!(vm.is_match("abcd"), true);
        assert_eq!(vm.is_match("abc"), false);
        assert_eq!(vm.is_match("ad"), false);
        assert_eq!(vm.is_match("zabcd"), true);
        assert_eq!(vm.is_match("abcdz"), true);
    }
    {
        let src = "a(?:bc)";
        let vm = Nfa::new(src).unwrap();

        assert_eq!(vm.is_match("abc"), true);
        assert_eq!(vm.is_match("a"), false);
        assert_eq!(vm.is_match("zabc"), true);
        assert_eq!(vm.is_match("abcd"), true);
    }
    {
        let src = "a(?:bc(?:de)f)(?:gh)";
        let vm = Nfa::new(src).unwrap();

        assert_eq!(vm.is_match("abcdefgh"), true);
        assert_eq!(vm.is_match("abcdef"), false);
        assert_eq!(vm.is_match("abcgh"), false);
        assert_eq!(vm.is_match("agh"), false);
    }
}

#[test]
fn union() {
    let src = "abc|def|ghi";
    let vm = Nfa::new(src).unwrap();

    assert_eq!(vm.is_match("abc"), true);
    assert_eq!(vm.is_match("def"), true);
    assert_eq!(vm.is_match("ghi"), true);
    assert_eq!(vm.is_match("adg"), false);
    assert_eq!(vm.is_match("ab"), false);
    assert_eq!(vm.is_match("zabc"), true);
    assert_eq!(vm.is_match("defz"), true);
}

#[cfg(test)]
mod greedy {
    use super::*;

    #[test]
    fn star() {
        {
            let src = "ab*c";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.is_match("ac"), true);
            assert_eq!(vm.is_match("abc"), true);
            assert_eq!(vm.is_match("abbc"), true);
            assert_eq!(vm.is_match("abbbc"), true);
            assert_eq!(vm.is_match("az"), false);
            assert_eq!(vm.is_match("zac"), true);
            assert_eq!(vm.is_match("acz"), true);
        }
        {
            let src = "ab*";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.is_match("a"), true);
            assert_eq!(vm.is_match("ab"), true);
            assert_eq!(vm.is_match("abb"), true);
            assert_eq!(vm.is_match("abbb"), true);
            assert_eq!(vm.is_match("b"), false);
            assert_eq!(vm.is_match("za"), true);
            assert_eq!(vm.is_match("az"), true);
        }
        {
            let src = "ab*b*";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.is_match("a"), true);
            assert_eq!(vm.is_match("ab"), true);
            assert_eq!(vm.is_match("abb"), true);
            assert_eq!(vm.is_match("abbb"), true);
            assert_eq!(vm.is_match("b"), false);
            assert_eq!(vm.is_match("za"), true);
            assert_eq!(vm.is_match("az"), true);
        }
        {
            let src = "a.*b";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.is_match("ab"), true);
            assert_eq!(vm.is_match("axb"), true);
            assert_eq!(vm.is_match("axbaxb"), true);
            #[rustfmt::skip]
            assert_eq!(vm.is_match("axaxbxb"), true);
            assert_eq!(vm.is_match("baxb"), true);
            assert_eq!(vm.is_match("axbz"), true);
        }
    }

    #[test]
    fn plus() {
        {
            let src = "ab+c";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.is_match("abc"), true);
            assert_eq!(vm.is_match("abbc"), true);
            assert_eq!(vm.is_match("abbbc"), true);
            assert_eq!(vm.is_match("ac"), false);
            assert_eq!(vm.is_match("zabc"), true);
            assert_eq!(vm.is_match("abcz"), true);
        }
        {
            let src = "ab+";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.is_match("ab"), true);
            assert_eq!(vm.is_match("abb"), true);
            assert_eq!(vm.is_match("abbb"), true);
            assert_eq!(vm.is_match("a"), false);
            assert_eq!(vm.is_match("zab"), true);
            assert_eq!(vm.is_match("abz"), true);
        }
        {
            let src = "ab+b+";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.is_match("abb"), true);
            assert_eq!(vm.is_match("abbb"), true);
            assert_eq!(vm.is_match("abbbb"), true);
            assert_eq!(vm.is_match("a"), false);
            assert_eq!(vm.is_match("ab"), false);
            assert_eq!(vm.is_match("zabb"), true);
            assert_eq!(vm.is_match("abbz"), true);
        }
        {
            let src = "a.+b";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.is_match("ab"), false);
            assert_eq!(vm.is_match("axb"), true);
            assert_eq!(vm.is_match("axbaxb"), true);
            assert_eq!(vm.is_match("axaxbxb"), true);
            assert_eq!(vm.is_match("baxb"), true);
            assert_eq!(vm.is_match("axbz"), true);
        }
    }

    #[test]
    fn option() {
        {
            let src = "ab?c";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.is_match("ac"), true);
            assert_eq!(vm.is_match("abc"), true);
            assert_eq!(vm.is_match("a"), false);
            assert_eq!(vm.is_match("zac"), true);
            assert_eq!(vm.is_match("acz"), true);
        }
        {
            let src = "ab?";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.is_match("a"), true);
            assert_eq!(vm.is_match("ab"), true);
            assert_eq!(vm.is_match("b"), false);
            assert_eq!(vm.is_match("za"), true);
            assert_eq!(vm.is_match("az"), true);
        }
    }

    #[test]
    fn repeat() {
        {
            let src = "a{3}";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.is_match("aaa"), true);
            assert_eq!(vm.is_match("aaaaa"), true);
            assert_eq!(vm.is_match("aa"), false);
            assert_eq!(vm.is_match("zaaa"), true);
            assert_eq!(vm.is_match("aaaz"), true);
        }
        {
            let src = "abc{3}";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.is_match("abccc"), true);
            assert_eq!(vm.is_match("abccccc"), true);
            assert_eq!(vm.is_match("abc"), false);
            assert_eq!(vm.is_match("zabccc"), true);
            assert_eq!(vm.is_match("abcccz"), true);
        }
        {
            let src = "(abc){3}";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.is_match("abcabcabc"), true);
            assert_eq!(vm.is_match("abcabc"), false);
            assert_eq!(vm.is_match("zabcabcabc"), true);
            assert_eq!(vm.is_match("abcabcabcz"), true);
        }
    }

    #[test]
    fn repeat_min() {
        {
            let src = "a{2,}";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.is_match("aa"), true);
            assert_eq!(vm.is_match("aaa"), true);
            assert_eq!(vm.is_match("a"), false);
            assert_eq!(vm.is_match("zaaa"), true);
            assert_eq!(vm.is_match("aaaz"), true);
        }
        {
            let src = "abc{2,}";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.is_match("abcc"), true);
            assert_eq!(vm.is_match("abccc"), true);
            assert_eq!(vm.is_match("abc"), false);
            assert_eq!(vm.is_match("zabcc"), true);
            assert_eq!(vm.is_match("abccz"), true);
        }
        {
            let src = "(abc){2,}";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.is_match("abcabc"), true);
            assert_eq!(vm.is_match("abcabcabc"), true);
            assert_eq!(vm.is_match("abc"), false);
            assert_eq!(vm.is_match("zabcabc"), true);
            assert_eq!(vm.is_match("abcabcz"), true);
        }
    }

    #[test]
    fn repeat_range() {
        {
            let src = "a{2,3}";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.is_match("aa"), true);
            assert_eq!(vm.is_match("aaa"), true);
            assert_eq!(vm.is_match("aaaa"), true);
            assert_eq!(vm.is_match("a"), false);
            assert_eq!(vm.is_match("zaa"), true);
            assert_eq!(vm.is_match("aaz"), true);
        }
        {
            let src = "abc{2,3}";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.is_match("abcc"), true);
            assert_eq!(vm.is_match("abccc"), true);
            assert_eq!(vm.is_match("abcccc"), true);
            assert_eq!(vm.is_match("abc"), false);
            assert_eq!(vm.is_match("zabcc"), true);
            assert_eq!(vm.is_match("abccz"), true);
        }
        {
            let src = "(abc){2,3}";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.is_match("abcabc"), true);
            assert_eq!(vm.is_match("abcabcabc"), true);
            assert_eq!(vm.is_match("abcabcabcabc"), true);
            assert_eq!(vm.is_match("abc"), false);
            assert_eq!(vm.is_match("zabcabc"), true);
            assert_eq!(vm.is_match("abcabcz"), true);
        }
    }
}

#[cfg(test)]
mod non_greedy {
    use super::*;

    #[test]
    fn star() {
        {
            let src = "ab*?c";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.is_match("ac"), true);
            assert_eq!(vm.is_match("abc"), true);
            assert_eq!(vm.is_match("abbc"), true);
            assert_eq!(vm.is_match("abbbc"), true);
            assert_eq!(vm.is_match("az"), false);
            assert_eq!(vm.is_match("zac"), true);
            assert_eq!(vm.is_match("acz"), true);
        }
        {
            let src = "ab*?";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.is_match("a"), true);
            assert_eq!(vm.is_match("ab"), true);
            assert_eq!(vm.is_match("abb"), true);
            assert_eq!(vm.is_match("abbb"), true);
            assert_eq!(vm.is_match("b"), false);
            assert_eq!(vm.is_match("za"), true);
            assert_eq!(vm.is_match("az"), true);
        }
        {
            let src = "ab*?b*?";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.is_match("a"), true);
            assert_eq!(vm.is_match("ab"), true);
            assert_eq!(vm.is_match("abb"), true);
            assert_eq!(vm.is_match("abbb"), true);
            assert_eq!(vm.is_match("b"), false);
            assert_eq!(vm.is_match("za"), true);
            assert_eq!(vm.is_match("az"), true);
        }
        {
            let src = "a.*?b";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.is_match("ab"), true);
            assert_eq!(vm.is_match("axb"), true);
            assert_eq!(vm.is_match("axbaxb"), true);
            #[rustfmt::skip]
            assert_eq!(vm.is_match("axaxbxb"), true);
            assert_eq!(vm.is_match("baxb"), true);
            assert_eq!(vm.is_match("axbz"), true);
        }
    }

    #[test]
    fn plus() {
        {
            let src = "ab+?c";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.is_match("abc"), true);
            assert_eq!(vm.is_match("abbc"), true);
            assert_eq!(vm.is_match("abbbc"), true);
            assert_eq!(vm.is_match("ac"), false);
            assert_eq!(vm.is_match("zabc"), true);
            assert_eq!(vm.is_match("abcz"), true);
        }
        {
            let src = "ab+?";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.is_match("ab"), true);
            assert_eq!(vm.is_match("abb"), true);
            assert_eq!(vm.is_match("abbb"), true);
            assert_eq!(vm.is_match("a"), false);
            assert_eq!(vm.is_match("zab"), true);
            assert_eq!(vm.is_match("abz"), true);
        }
        {
            let src = "ab+?b+?";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.is_match("abb"), true);
            assert_eq!(vm.is_match("abbb"), true);
            assert_eq!(vm.is_match("abbbb"), true);
            assert_eq!(vm.is_match("a"), false);
            assert_eq!(vm.is_match("ab"), false);
            assert_eq!(vm.is_match("zabb"), true);
            assert_eq!(vm.is_match("abbz"), true);
        }
        {
            let src = "a.+?b";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.is_match("ab"), false);
            assert_eq!(vm.is_match("axb"), true);
            assert_eq!(vm.is_match("axbaxb"), true);
            assert_eq!(vm.is_match("axaxbxb"), true);
            assert_eq!(vm.is_match("baxb"), true);
            assert_eq!(vm.is_match("axbz"), true);
        }
    }

    #[test]
    fn option() {
        {
            let src = "ab??c";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.is_match("ac"), true);
            assert_eq!(vm.is_match("abc"), true);
            assert_eq!(vm.is_match("a"), false);
            assert_eq!(vm.is_match("zac"), true);
            assert_eq!(vm.is_match("acz"), true);
        }
        {
            let src = "ab??";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.is_match("a"), true);
            assert_eq!(vm.is_match("ab"), true);
            assert_eq!(vm.is_match("b"), false);
            assert_eq!(vm.is_match("za"), true);
            assert_eq!(vm.is_match("az"), true);
        }
    }

    #[test]
    fn repeat() {
        {
            let src = "a{3}?";
            let _ = Nfa::new(src).unwrap();

            // show warning error
        }
    }

    #[test]
    fn repeat_min() {
        {
            let src = "a{2,}?";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.is_match("aa"), true);
            assert_eq!(vm.is_match("aaa"), true);
            assert_eq!(vm.is_match("a"), false);
            assert_eq!(vm.is_match("zaaa"), true);
            assert_eq!(vm.is_match("aaaz"), true);
        }
        {
            let src = "abc{2,}?";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.is_match("abcc"), true);
            assert_eq!(vm.is_match("abccc"), true);
            assert_eq!(vm.is_match("abc"), false);
            assert_eq!(vm.is_match("zabcc"), true);
            assert_eq!(vm.is_match("abccz"), true);
        }
        {
            let src = "(abc){2,}?";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.is_match("abcabc"), true);
            assert_eq!(vm.is_match("abcabcabc"), true);
            assert_eq!(vm.is_match("abc"), false);
            assert_eq!(vm.is_match("zabcabc"), true);
            assert_eq!(vm.is_match("abcabcz"), true);
        }
    }

    #[test]
    fn repeat_range() {
        {
            let src = "a{2,3}?";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.is_match("aa"), true);
            assert_eq!(vm.is_match("aaa"), true);
            assert_eq!(vm.is_match("aaaa"), true);
            assert_eq!(vm.is_match("a"), false);
            assert_eq!(vm.is_match("zaa"), true);
            assert_eq!(vm.is_match("aaz"), true);
        }
        {
            let src = "abc{2,3}?";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.is_match("abcc"), true);
            assert_eq!(vm.is_match("abccc"), true);
            assert_eq!(vm.is_match("abcccc"), true);
            assert_eq!(vm.is_match("abc"), false);
            assert_eq!(vm.is_match("zabcc"), true);
            assert_eq!(vm.is_match("abccz"), true);
        }
        {
            let src = "(abc){2,3}?";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.is_match("abcabc"), true);
            assert_eq!(vm.is_match("abcabcabc"), true);
            assert_eq!(vm.is_match("abcabcabcabc"), true);
            assert_eq!(vm.is_match("abc"), false);
            assert_eq!(vm.is_match("zabcabc"), true);
            assert_eq!(vm.is_match("abcabcz"), true);
        }
    }
}

#[cfg(test)]
mod set {
    use super::*;

    #[test]
    fn positive() {
        {
            let src = "a[b-z]d";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.is_match("abd"), true);
            assert_eq!(vm.is_match("azd"), true);
            assert_eq!(vm.is_match("axd"), true);
            assert_eq!(vm.is_match("ad"), false);
            assert_eq!(vm.is_match("aad"), false);
            assert_eq!(vm.is_match("zabd"), true);
            assert_eq!(vm.is_match("abdz"), true);
        }
        {
            let src = "[b-z]";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.is_match("b"), true);
            assert_eq!(vm.is_match("z"), true);
            assert_eq!(vm.is_match("x"), true);
            assert_eq!(vm.is_match("a"), false);
            assert_eq!(vm.is_match("ab"), true);
            assert_eq!(vm.is_match("bz"), true);
        }
        {
            let src = "[bcd]";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.is_match("b"), true);
            assert_eq!(vm.is_match("c"), true);
            assert_eq!(vm.is_match("d"), true);
            assert_eq!(vm.is_match("a"), false);
            assert_eq!(vm.is_match("e"), false);
            assert_eq!(vm.is_match("ab"), true);
            assert_eq!(vm.is_match("bz"), true);
        }
        {
            let src = "a[bc-yz]d";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.is_match("abd"), true);
            assert_eq!(vm.is_match("azd"), true);
            assert_eq!(vm.is_match("acd"), true);
            assert_eq!(vm.is_match("ayd"), true);
            assert_eq!(vm.is_match("axd"), true);
            assert_eq!(vm.is_match("aad"), false);
            assert_eq!(vm.is_match("ad"), false);
            assert_eq!(vm.is_match("zabd"), true);
            assert_eq!(vm.is_match("abdz"), true);
        }
        {
            let src = "[z-z]";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.is_match("z"), true);
            assert_eq!(vm.is_match("a"), false);
            assert_eq!(vm.is_match("az"), true);
            assert_eq!(vm.is_match("za"), true);
        }
    }

    #[test]
    fn negative() {
        {
            let src = "a[^b-z]d";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.is_match("abd"), false);
            assert_eq!(vm.is_match("azd"), false);
            assert_eq!(vm.is_match("axd"), false);
            assert_eq!(vm.is_match("aad"), true);
            assert_eq!(vm.is_match("ad"), false);
            assert_eq!(vm.is_match("zaad"), true);
            assert_eq!(vm.is_match("aadz"), true);
        }
        {
            let src = "[^b-z]";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.is_match("b"), false);
            assert_eq!(vm.is_match("z"), false);
            assert_eq!(vm.is_match("x"), false);
            assert_eq!(vm.is_match("a"), true);
            assert_eq!(vm.is_match("za"), true);
            assert_eq!(vm.is_match("az"), true);
        }
        {
            let src = "[^bcd]";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.is_match("b"), false);
            assert_eq!(vm.is_match("c"), false);
            assert_eq!(vm.is_match("d"), false);
            assert_eq!(vm.is_match("a"), true);
            assert_eq!(vm.is_match("e"), true);
            assert_eq!(vm.is_match("ba"), true);
            assert_eq!(vm.is_match("ab"), true);
        }
        {
            let src = "a[^bc-yz]d";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.is_match("abd"), false);
            assert_eq!(vm.is_match("azd"), false);
            assert_eq!(vm.is_match("acd"), false);
            assert_eq!(vm.is_match("ayd"), false);
            assert_eq!(vm.is_match("axd"), false);
            assert_eq!(vm.is_match("aad"), true);
            assert_eq!(vm.is_match("ad"), false);
            assert_eq!(vm.is_match("zaad"), true);
            assert_eq!(vm.is_match("aadz"), true);
        }
        {
            let src = "[^z-z]";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.is_match("z"), false);
            assert_eq!(vm.is_match("a"), true);
            assert_eq!(vm.is_match("za"), true);
            assert_eq!(vm.is_match("az"), true);
        }
    }
}

#[test]
fn pattern001() {
    {
        let src = r"[a-zA-Z0-9_\.\+\-]+@[a-zA-Z0-9_\.]+[a-zA-Z]+";
        let vm = Nfa::new(src).unwrap();

        assert_eq!(vm.is_match("abc@example.com"), true);
        assert_eq!(vm.is_match("abc+123@me.example.com"), true);
        assert_eq!(vm.is_match("abc@example"), true);
        assert_eq!(vm.is_match("abc@example.123"), true);
        assert_eq!(vm.is_match("abc@def@example.com"), true);
    }
    {
        let src = r"^[a-zA-Z0-9_\.\+\-]+@[a-zA-Z0-9_\.]+[a-zA-Z]+$";
        let vm = Nfa::new(src).unwrap();

        assert_eq!(vm.is_match("abc@example.com"), true);
        assert_eq!(vm.is_match("abc+123@me.example.com"), true);
        assert_eq!(vm.is_match("abc@example"), true);
        assert_eq!(vm.is_match("abc@example.123"), false);
        assert_eq!(vm.is_match("abc@def@example.com"), false);
    }
}
