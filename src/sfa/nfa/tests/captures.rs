use super::super::*;

#[cfg(test)]
mod basic_match {
    use super::*;

    #[test]
    fn match_char() {
        let src = "a(b)c";
        let vm = Nfa::new(src).unwrap();

        assert_eq!(vm.captures("abc"), vec!["abc", "b"]);
        assert_eq!(vm.captures("ab"), Vec::<&str>::new());
        assert_eq!(vm.captures("abcd"), vec!["abc", "b"]);
        assert_eq!(vm.captures("zabc"), vec!["abc", "b"]);
    }

    #[test]
    fn match_metachar() {
        let src = r"a(\+)c";
        let vm = Nfa::new(src).unwrap();

        assert_eq!(vm.captures("a+c"), vec!["a+c", "+"]);
        assert_eq!(vm.captures("aac"), Vec::<&str>::new());
        assert_eq!(vm.captures("ac"), Vec::<&str>::new());
        assert_eq!(vm.captures("a+cz"), vec!["a+c", "+"]);
        assert_eq!(vm.captures("za+c"), vec!["a+c", "+"]);
    }

    #[test]
    fn match_any() {
        {
            let src = "a(.)c";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.captures("abc"), vec!["abc", "b"]);
            assert_eq!(vm.captures("adc"), vec!["adc", "d"]);
            assert_eq!(vm.captures("ac"), Vec::<&str>::new());
            assert_eq!(vm.captures("abbc"), Vec::<&str>::new());
            assert_eq!(vm.captures("zabc"), vec!["abc", "b"]);
            assert_eq!(vm.captures("abcz"), vec!["abc", "b"]);
        }
        {
            let src = "a(.)";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.captures("ab"), vec!["ab", "b"]);
            assert_eq!(vm.captures("ad"), vec!["ad", "d"]);
            assert_eq!(vm.captures("a"), Vec::<&str>::new());
            assert_eq!(vm.captures("abz"), vec!["ab", "b"]);
            assert_eq!(vm.captures("zab"), vec!["ab", "b"]);
        }
    }

    #[test]
    fn match_sol() {
        {
            let src = "^(a)bc";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.captures("abc"), vec!["abc", "a"]);
            assert_eq!(vm.captures("zabc"), Vec::<&str>::new());
            assert_eq!(vm.captures("abcz"), vec!["abc", "a"]);
        }
    }

    #[test]
    fn match_eol() {
        {
            let src = "ab(c)$";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.captures("abc"), vec!["abc", "c"]);
            assert_eq!(vm.captures("zabc"), vec!["abc", "c"]);
            assert_eq!(vm.captures("abcz"), Vec::<&str>::new());
        }
    }
}

#[test]
fn capture_group() {
    {
        let src = "a(bc)d";
        let vm = Nfa::new(src).unwrap();

        assert_eq!(vm.captures("abcd"), vec!["abcd", "bc"]);
        assert_eq!(vm.captures("abc"), Vec::<&str>::new());
        assert_eq!(vm.captures("ad"), Vec::<&str>::new());
        assert_eq!(vm.captures("zabcd"), vec!["abcd", "bc"]);
        assert_eq!(vm.captures("abcdz"), vec!["abcd", "bc"]);
    }
    {
        let src = "a(bc)";
        let vm = Nfa::new(src).unwrap();

        assert_eq!(vm.captures("abc"), vec!["abc", "bc"]);
        assert_eq!(vm.captures("a"), Vec::<&str>::new());
        assert_eq!(vm.captures("zabc"), vec!["abc", "bc"]);
        assert_eq!(vm.captures("abcd"), vec!["abc", "bc"]);
    }
    {
        let src = "a(bc(de)f)(gh)";
        let vm = Nfa::new(src).unwrap();

        assert_eq!(
            vm.captures("abcdefgh"),
            vec!["abcdefgh", "bcdef", "de", "gh"]
        );
        assert_eq!(vm.captures("abcdef"), Vec::<&str>::new());
        assert_eq!(vm.captures("abcgh"), Vec::<&str>::new());
        assert_eq!(vm.captures("agh"), Vec::<&str>::new());
    }
}

#[test]
fn noncapture_group() {
    {
        let src = "a(?:bc)d";
        let vm = Nfa::new(src).unwrap();

        assert_eq!(vm.captures("abcd"), vec!["abcd"]);
        assert_eq!(vm.captures("abc"), Vec::<&str>::new());
        assert_eq!(vm.captures("ad"), Vec::<&str>::new());
        assert_eq!(vm.captures("zabcd"), vec!["abcd"]);
        assert_eq!(vm.captures("abcdz"), vec!["abcd"]);
    }
    {
        let src = "a(?:bc)";
        let vm = Nfa::new(src).unwrap();

        assert_eq!(vm.captures("abc"), vec!["abc"]);
        assert_eq!(vm.captures("a"), Vec::<&str>::new());
        assert_eq!(vm.captures("zabc"), vec!["abc"]);
        assert_eq!(vm.captures("abcd"), vec!["abc"]);
    }
    {
        let src = "a(?:bc(?:de)f)(?:gh)";
        let vm = Nfa::new(src).unwrap();

        assert_eq!(vm.captures("abcdefgh"), vec!["abcdefgh"]);
        assert_eq!(vm.captures("abcdef"), Vec::<&str>::new());
        assert_eq!(vm.captures("abcgh"), Vec::<&str>::new());
        assert_eq!(vm.captures("agh"), Vec::<&str>::new());
    }
}

#[test]
fn union() {
    let src = "abc|def|ghi";
    let vm = Nfa::new(src).unwrap();

    assert_eq!(vm.captures("abc"), vec!["abc"]);
    assert_eq!(vm.captures("def"), vec!["def"]);
    assert_eq!(vm.captures("ghi"), vec!["ghi"]);
    assert_eq!(vm.captures("adg"), Vec::<&str>::new());
    assert_eq!(vm.captures("ab"), Vec::<&str>::new());
    assert_eq!(vm.captures("zabc"), vec!["abc"]);
    assert_eq!(vm.captures("defz"), vec!["def"]);
}

#[cfg(test)]
mod greedy {
    use super::*;

    #[test]
    fn star() {
        {
            let src = "a(b*)c";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.captures("ac"), vec!["ac", ""]);
            assert_eq!(vm.captures("abc"), vec!["abc", "b"]);
            assert_eq!(vm.captures("abbc"), vec!["abbc", "bb"]);
            assert_eq!(vm.captures("abbbc"), vec!["abbbc", "bbb"]);
            assert_eq!(vm.captures("az"), Vec::<&str>::new());
            assert_eq!(vm.captures("zac"), vec!["ac", ""]);
            assert_eq!(vm.captures("acz"), vec!["ac", ""]);
        }
        {
            let src = "a(b*)";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.captures("a"), vec!["a", ""]);
            assert_eq!(vm.captures("ab"), vec!["ab", "b"]);
            assert_eq!(vm.captures("abb"), vec!["abb", "bb"]);
            assert_eq!(vm.captures("abbb"), vec!["abbb", "bbb"]);
            assert_eq!(vm.captures("b"), Vec::<&str>::new());
            assert_eq!(vm.captures("za"), vec!["a", ""]);
            assert_eq!(vm.captures("az"), vec!["a", ""]);
        }
        {
            let src = "a(b*)b*";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.captures("a"), vec!["a", ""]);
            assert_eq!(vm.captures("ab"), vec!["ab", "b"]);
            assert_eq!(vm.captures("abb"), vec!["abb", "bb"]);
            assert_eq!(vm.captures("abbb"), vec!["abbb", "bbb"]);
            assert_eq!(vm.captures("b"), Vec::<&str>::new());
            assert_eq!(vm.captures("za"), vec!["a", ""]);
            assert_eq!(vm.captures("az"), vec!["a", ""]);
        }
        {
            let src = "a(.*)b";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.captures("ab"), vec!["ab", ""]);
            assert_eq!(vm.captures("axb"), vec!["axb", "x"]);
            assert_eq!(vm.captures("axbaxb"), vec!["axbaxb", "xbax"]);
            assert_eq!(vm.captures("axaxbxb"), vec!["axaxbxb", "xaxbx"]);
            assert_eq!(vm.captures("baxb"), vec!["axb", "x"]);
            assert_eq!(vm.captures("axbz"), vec!["axb", "x"]);
        }
    }

    #[test]
    fn plus() {
        {
            let src = "a(b+)c";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.captures("abc"), vec!["abc", "b"]);
            assert_eq!(vm.captures("abbc"), vec!["abbc", "bb"]);
            assert_eq!(vm.captures("abbbc"), vec!["abbbc", "bbb"]);
            assert_eq!(vm.captures("ac"), Vec::<&str>::new());
            assert_eq!(vm.captures("zabc"), vec!["abc", "b"]);
            assert_eq!(vm.captures("abcz"), vec!["abc", "b"]);
        }
        {
            let src = "a(b+)";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.captures("ab"), vec!["ab", "b"]);
            assert_eq!(vm.captures("abb"), vec!["abb", "bb"]);
            assert_eq!(vm.captures("abbb"), vec!["abbb", "bbb"]);
            assert_eq!(vm.captures("a"), Vec::<&str>::new());
            assert_eq!(vm.captures("zab"), vec!["ab", "b"]);
            assert_eq!(vm.captures("abz"), vec!["ab", "b"]);
        }
        {
            let src = "a(b+)b+";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.captures("abb"), vec!["abb", "b"]);
            assert_eq!(vm.captures("abbb"), vec!["abbb", "bb"]);
            assert_eq!(vm.captures("abbbb"), vec!["abbbb", "bbb"]);
            assert_eq!(vm.captures("a"), Vec::<&str>::new());
            assert_eq!(vm.captures("ab"), Vec::<&str>::new());
            assert_eq!(vm.captures("zabb"), vec!["abb", "b"]);
            assert_eq!(vm.captures("abbz"), vec!["abb", "b"]);
        }
        {
            let src = "a(.+)b";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.captures("ab"), Vec::<&str>::new());
            assert_eq!(vm.captures("axb"), vec!["axb", "x"]);
            assert_eq!(vm.captures("axbaxb"), vec!["axbaxb", "xbax"]);
            assert_eq!(vm.captures("axaxbxb"), vec!["axaxbxb", "xaxbx"]);
            assert_eq!(vm.captures("baxb"), vec!["axb", "x"]);
            assert_eq!(vm.captures("axbz"), vec!["axb", "x"]);
        }
    }

    #[test]
    fn option() {
        {
            let src = "a(b?)c";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.captures("ac"), vec!["ac", ""]);
            assert_eq!(vm.captures("abc"), vec!["abc", "b"]);
            assert_eq!(vm.captures("a"), Vec::<&str>::new());
            assert_eq!(vm.captures("zac"), vec!["ac", ""]);
            assert_eq!(vm.captures("acz"), vec!["ac", ""]);
        }
        {
            let src = "a(b?)";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.captures("a"), vec!["a", ""]);
            assert_eq!(vm.captures("ab"), vec!["ab", "b"]);
            assert_eq!(vm.captures("b"), Vec::<&str>::new());
            assert_eq!(vm.captures("za"), vec!["a", ""]);
            assert_eq!(vm.captures("az"), vec!["a", ""]);
        }
    }

    #[test]
    fn repeat() {
        {
            let src = "(a{3})";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.captures("aaa"), vec!["aaa", "aaa"]);
            assert_eq!(vm.captures("aaaaa"), vec!["aaa", "aaa"]);
            assert_eq!(vm.captures("aa"), Vec::<&str>::new());
            assert_eq!(vm.captures("zaaa"), vec!["aaa", "aaa"]);
            assert_eq!(vm.captures("aaaz"), vec!["aaa", "aaa"]);
        }
        {
            let src = "abc{3}";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.captures("abccc"), vec!["abccc"]);
            assert_eq!(vm.captures("abccccc"), vec!["abccc"]);
            assert_eq!(vm.captures("abc"), Vec::<&str>::new());
            assert_eq!(vm.captures("zabccc"), vec!["abccc"]);
            assert_eq!(vm.captures("abcccz"), vec!["abccc"]);
        }
        {
            let src = "(abc){3}";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.captures("abcabcabc"), vec!["abcabcabc", "abc"]);
            assert_eq!(vm.captures("abcabc"), Vec::<&str>::new());
            assert_eq!(vm.captures("zabcabcabc"), vec!["abcabcabc", "abc"]);
            assert_eq!(vm.captures("abcabcabcz"), vec!["abcabcabc", "abc"]);
        }
    }

    #[test]
    fn repeat_min() {
        {
            let src = "a{2,}";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.captures("aa"), vec!["aa"]);
            assert_eq!(vm.captures("aaa"), vec!["aaa"]);
            assert_eq!(vm.captures("a"), Vec::<&str>::new());
            assert_eq!(vm.captures("zaaa"), vec!["aaa"]);
            assert_eq!(vm.captures("aaaz"), vec!["aaa"]);
        }
        {
            let src = "abc{2,}";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.captures("abcc"), vec!["abcc"]);
            assert_eq!(vm.captures("abccc"), vec!["abccc"]);
            assert_eq!(vm.captures("abc"), Vec::<&str>::new());
            assert_eq!(vm.captures("zabcc"), vec!["abcc"]);
            assert_eq!(vm.captures("abccz"), vec!["abcc"]);
        }
        {
            let src = "(abc){2,}";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.captures("abcabc"), vec!["abcabc", "abc"]);
            assert_eq!(vm.captures("abcabcabc"), vec!["abcabcabc", "abc"]);
            assert_eq!(vm.captures("abc"), Vec::<&str>::new());
            assert_eq!(vm.captures("zabcabc"), vec!["abcabc", "abc"]);
            assert_eq!(vm.captures("abcabcz"), vec!["abcabc", "abc"]);
        }
    }

    #[test]
    fn repeat_range() {
        {
            let src = "a{2,3}";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.captures("aa"), vec!["aa"]);
            assert_eq!(vm.captures("aaa"), vec!["aaa"]);
            assert_eq!(vm.captures("aaaa"), vec!["aaa"]);
            assert_eq!(vm.captures("a"), Vec::<&str>::new());
            assert_eq!(vm.captures("zaa"), vec!["aa"]);
            assert_eq!(vm.captures("aaz"), vec!["aa"]);
        }
        {
            let src = "abc{2,3}";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.captures("abcc"), vec!["abcc"]);
            assert_eq!(vm.captures("abccc"), vec!["abccc"]);
            assert_eq!(vm.captures("abcccc"), vec!["abccc"]);
            assert_eq!(vm.captures("abc"), Vec::<&str>::new());
            assert_eq!(vm.captures("zabcc"), vec!["abcc"]);
            assert_eq!(vm.captures("abccz"), vec!["abcc"]);
        }
        {
            let src = "(abc){2,3}";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.captures("abcabc"), vec!["abcabc", "abc"]);
            assert_eq!(vm.captures("abcabcabc"), vec!["abcabcabc", "abc"]);
            assert_eq!(vm.captures("abcabcabcabc"), vec!["abcabcabc", "abc"]);
            assert_eq!(vm.captures("abc"), Vec::<&str>::new());
            assert_eq!(vm.captures("zabcabc"), vec!["abcabc", "abc"]);
            assert_eq!(vm.captures("abcabcz"), vec!["abcabc", "abc"]);
        }
    }
}

#[cfg(test)]
mod non_greedy {
    use super::*;

    #[test]
    fn star() {
        {
            let src = "a(b*?)c";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.captures("ac"), vec!["ac", ""]);
            assert_eq!(vm.captures("abc"), vec!["abc", "b"]);
            assert_eq!(vm.captures("abbc"), vec!["abbc", "bb"]);
            assert_eq!(vm.captures("abbbc"), vec!["abbbc", "bbb"]);
            assert_eq!(vm.captures("az"), Vec::<&str>::new());
            assert_eq!(vm.captures("zac"), vec!["ac", ""]);
            assert_eq!(vm.captures("acz"), vec!["ac", ""]);
        }
        {
            let src = "a(b*?)";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.captures("a"), vec!["a", ""]);
            assert_eq!(vm.captures("ab"), vec!["a", ""]);
            assert_eq!(vm.captures("abb"), vec!["a", ""]);
            assert_eq!(vm.captures("abbb"), vec!["a", ""]);
            assert_eq!(vm.captures("b"), Vec::<&str>::new());
            assert_eq!(vm.captures("za"), vec!["a", ""]);
            assert_eq!(vm.captures("az"), vec!["a", ""]);
        }
        {
            let src = "a(b*?)b*?";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.captures("a"), vec!["a", ""]);
            assert_eq!(vm.captures("ab"), vec!["a", ""]);
            assert_eq!(vm.captures("abb"), vec!["a", ""]);
            assert_eq!(vm.captures("abbb"), vec!["a", ""]);
            assert_eq!(vm.captures("b"), Vec::<&str>::new());
            assert_eq!(vm.captures("za"), vec!["a", ""]);
            assert_eq!(vm.captures("az"), vec!["a", ""]);
        }
        {
            let src = "a(.*?)b";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.captures("ab"), vec!["ab", ""]);
            assert_eq!(vm.captures("axb"), vec!["axb", "x"]);
            assert_eq!(vm.captures("axbaxb"), vec!["axb", "x"]);
            assert_eq!(vm.captures("axaxbxb"), vec!["axaxb", "xax"]);
            assert_eq!(vm.captures("baxb"), vec!["axb", "x"]);
            assert_eq!(vm.captures("axbz"), vec!["axb", "x"]);
        }
    }

    #[test]
    fn plus() {
        {
            let src = "a(b+?)c";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.captures("abc"), vec!["abc", "b"]);
            assert_eq!(vm.captures("abbc"), vec!["abbc", "bb"]);
            assert_eq!(vm.captures("abbbc"), vec!["abbbc", "bbb"]);
            assert_eq!(vm.captures("ac"), Vec::<&str>::new());
            assert_eq!(vm.captures("zabc"), vec!["abc", "b"]);
            assert_eq!(vm.captures("abcz"), vec!["abc", "b"]);
        }
        {
            let src = "a(b+?)";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.captures("ab"), vec!["ab", "b"]);
            assert_eq!(vm.captures("abb"), vec!["ab", "b"]);
            assert_eq!(vm.captures("abbb"), vec!["ab", "b"]);
            assert_eq!(vm.captures("a"), Vec::<&str>::new());
            assert_eq!(vm.captures("zab"), vec!["ab", "b"]);
            assert_eq!(vm.captures("abz"), vec!["ab", "b"]);
        }
        {
            let src = "a(b+?)b+?";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.captures("abb"), vec!["abb", "b"]);
            assert_eq!(vm.captures("abbb"), vec!["abb", "b"]);
            assert_eq!(vm.captures("abbbb"), vec!["abb", "b"]);
            assert_eq!(vm.captures("a"), Vec::<&str>::new());
            assert_eq!(vm.captures("ab"), Vec::<&str>::new());
            assert_eq!(vm.captures("zabb"), vec!["abb", "b"]);
            assert_eq!(vm.captures("abbz"), vec!["abb", "b"]);
        }
        {
            let src = "a(.+?)b";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.captures("ab"), Vec::<&str>::new());
            assert_eq!(vm.captures("axb"), vec!["axb", "x"]);
            assert_eq!(vm.captures("axbaxb"), vec!["axb", "x"]);
            assert_eq!(vm.captures("axaxbxb"), vec!["axaxb", "xax"]);
            assert_eq!(vm.captures("baxb"), vec!["axb", "x"]);
            assert_eq!(vm.captures("axbz"), vec!["axb", "x"]);
        }
    }

    #[test]
    fn option() {
        {
            let src = "a(b??)c";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.captures("ac"), vec!["ac", ""]);
            assert_eq!(vm.captures("abc"), vec!["abc", "b"]);
            assert_eq!(vm.captures("a"), Vec::<&str>::new());
            assert_eq!(vm.captures("zac"), vec!["ac", ""]);
            assert_eq!(vm.captures("acz"), vec!["ac", ""]);
        }
        {
            let src = "a(b??)";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.captures("a"), vec!["a", ""]);
            assert_eq!(vm.captures("ab"), vec!["a", ""]);
            assert_eq!(vm.captures("b"), Vec::<&str>::new());
            assert_eq!(vm.captures("za"), vec!["a", ""]);
            assert_eq!(vm.captures("az"), vec!["a", ""]);
        }
    }

    #[test]
    fn repeat_min() {
        {
            let src = "a{2,}?";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.captures("aa"), vec!["aa"]);
            assert_eq!(vm.captures("aaa"), vec!["aa"]);
            assert_eq!(vm.captures("a"), Vec::<&str>::new());
            assert_eq!(vm.captures("zaaa"), vec!["aa"]);
            assert_eq!(vm.captures("aaaz"), vec!["aa"]);
        }
        {
            let src = "abc{2,}?";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.captures("abcc"), vec!["abcc"]);
            assert_eq!(vm.captures("abccc"), vec!["abcc"]);
            assert_eq!(vm.captures("abc"), Vec::<&str>::new());
            assert_eq!(vm.captures("zabcc"), vec!["abcc"]);
            assert_eq!(vm.captures("abccz"), vec!["abcc"]);
        }
        {
            let src = "(abc){2,}?";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.captures("abcabc"), vec!["abcabc", "abc"]);
            assert_eq!(vm.captures("abcabcabc"), vec!["abcabc", "abc"]);
            assert_eq!(vm.captures("abc"), Vec::<&str>::new());
            assert_eq!(vm.captures("zabcabc"), vec!["abcabc", "abc"]);
            assert_eq!(vm.captures("abcabcz"), vec!["abcabc", "abc"]);
        }
    }

    #[test]
    fn repeat_range() {
        {
            let src = "a{2,3}?";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.captures("aa"), vec!["aa"]);
            assert_eq!(vm.captures("aaa"), vec!["aa"]);
            assert_eq!(vm.captures("aaaa"), vec!["aa"]);
            assert_eq!(vm.captures("a"), Vec::<&str>::new());
            assert_eq!(vm.captures("zaa"), vec!["aa"]);
            assert_eq!(vm.captures("aaz"), vec!["aa"]);
        }
        {
            let src = "abc{2,3}?";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.captures("abcc"), vec!["abcc"]);
            assert_eq!(vm.captures("abccc"), vec!["abcc"]);
            assert_eq!(vm.captures("abcccc"), vec!["abcc"]);
            assert_eq!(vm.captures("abc"), Vec::<&str>::new());
            assert_eq!(vm.captures("zabcc"), vec!["abcc"]);
            assert_eq!(vm.captures("abccz"), vec!["abcc"]);
        }
        {
            let src = "(abc){2,3}?";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.captures("abcabc"), vec!["abcabc", "abc"]);
            assert_eq!(vm.captures("abcabcabc"), vec!["abcabc", "abc"]);
            assert_eq!(vm.captures("abcabcabcabc"), vec!["abcabc", "abc"]);
            assert_eq!(vm.captures("abc"), Vec::<&str>::new());
            assert_eq!(vm.captures("zabcabc"), vec!["abcabc", "abc"]);
            assert_eq!(vm.captures("abcabcz"), vec!["abcabc", "abc"]);
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

            assert_eq!(vm.captures("abd"), vec!["abd"]);
            assert_eq!(vm.captures("azd"), vec!["azd"]);
            assert_eq!(vm.captures("axd"), vec!["axd"]);
            assert_eq!(vm.captures("ad"), Vec::<&str>::new());
            assert_eq!(vm.captures("aad"), Vec::<&str>::new());
            assert_eq!(vm.captures("zabd"), vec!["abd"]);
            assert_eq!(vm.captures("abdz"), vec!["abd"]);
        }
        {
            let src = "[b-z]";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.captures("b"), vec!["b"]);
            assert_eq!(vm.captures("z"), vec!["z"]);
            assert_eq!(vm.captures("x"), vec!["x"]);
            assert_eq!(vm.captures("a"), Vec::<&str>::new());
            assert_eq!(vm.captures("ab"), vec!["b"]);
            assert_eq!(vm.captures("bz"), vec!["b"]);
        }
        {
            let src = "[bcd]";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.captures("b"), vec!["b"]);
            assert_eq!(vm.captures("c"), vec!["c"]);
            assert_eq!(vm.captures("d"), vec!["d"]);
            assert_eq!(vm.captures("a"), Vec::<&str>::new());
            assert_eq!(vm.captures("e"), Vec::<&str>::new());
            assert_eq!(vm.captures("ab"), vec!["b"]);
            assert_eq!(vm.captures("bz"), vec!["b"]);
        }
        {
            let src = "a[bc-yz]d";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.captures("abd"), vec!["abd"]);
            assert_eq!(vm.captures("azd"), vec!["azd"]);
            assert_eq!(vm.captures("acd"), vec!["acd"]);
            assert_eq!(vm.captures("ayd"), vec!["ayd"]);
            assert_eq!(vm.captures("axd"), vec!["axd"]);
            assert_eq!(vm.captures("aad"), Vec::<&str>::new());
            assert_eq!(vm.captures("ad"), Vec::<&str>::new());
            assert_eq!(vm.captures("zabd"), vec!["abd"]);
            assert_eq!(vm.captures("abdz"), vec!["abd"]);
        }
        {
            let src = "[z-z]";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.captures("z"), vec!["z"]);
            assert_eq!(vm.captures("a"), Vec::<&str>::new());
            assert_eq!(vm.captures("az"), vec!["z"]);
            assert_eq!(vm.captures("za"), vec!["z"]);
        }
    }

    #[test]
    fn negative() {
        {
            let src = "a[^b-z]d";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.captures("abd"), Vec::<&str>::new());
            assert_eq!(vm.captures("azd"), Vec::<&str>::new());
            assert_eq!(vm.captures("axd"), Vec::<&str>::new());
            assert_eq!(vm.captures("aad"), vec!["aad"]);
            assert_eq!(vm.captures("ad"), Vec::<&str>::new());
            assert_eq!(vm.captures("zaad"), vec!["aad"]);
            assert_eq!(vm.captures("aadz"), vec!["aad"]);
        }
        {
            let src = "[^b-z]";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.captures("b"), Vec::<&str>::new());
            assert_eq!(vm.captures("z"), Vec::<&str>::new());
            assert_eq!(vm.captures("x"), Vec::<&str>::new());
            assert_eq!(vm.captures("a"), vec!["a"]);
            assert_eq!(vm.captures("za"), vec!["a"]);
            assert_eq!(vm.captures("az"), vec!["a"]);
        }
        {
            let src = "[^bcd]";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.captures("b"), Vec::<&str>::new());
            assert_eq!(vm.captures("c"), Vec::<&str>::new());
            assert_eq!(vm.captures("d"), Vec::<&str>::new());
            assert_eq!(vm.captures("a"), vec!["a"]);
            assert_eq!(vm.captures("e"), vec!["e"]);
            assert_eq!(vm.captures("ba"), vec!["a"]);
            assert_eq!(vm.captures("ab"), vec!["a"]);
        }
        {
            let src = "a[^bc-yz]d";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.captures("abd"), Vec::<&str>::new());
            assert_eq!(vm.captures("azd"), Vec::<&str>::new());
            assert_eq!(vm.captures("acd"), Vec::<&str>::new());
            assert_eq!(vm.captures("ayd"), Vec::<&str>::new());
            assert_eq!(vm.captures("axd"), Vec::<&str>::new());
            assert_eq!(vm.captures("aad"), vec!["aad"]);
            assert_eq!(vm.captures("ad"), Vec::<&str>::new());
            assert_eq!(vm.captures("zaad"), vec!["aad"]);
            assert_eq!(vm.captures("aadz"), vec!["aad"]);
        }
        {
            let src = "[^z-z]";
            let vm = Nfa::new(src).unwrap();

            assert_eq!(vm.captures("z"), Vec::<&str>::new());
            assert_eq!(vm.captures("a"), vec!["a"]);
            assert_eq!(vm.captures("za"), vec!["a"]);
            assert_eq!(vm.captures("az"), vec!["a"]);
        }
    }
}

#[test]
fn pattern001() {
    {
        let src = r"[a-zA-Z0-9_\.\+\-]+@[a-zA-Z0-9_\.]+[a-zA-Z]+";
        let vm = Nfa::new(src).unwrap();

        assert_eq!(vm.captures("abc@example.com"), vec!["abc@example.com"]);
        assert_eq!(
            vm.captures("abc+123@me.example.com"),
            vec!["abc+123@me.example.com"]
        );
        assert_eq!(vm.captures("abc@example"), vec!["abc@example"]);
        assert_eq!(vm.captures("abc@example.123"), vec!["abc@example"]);
        assert_eq!(vm.captures("abc@def@example.com"), vec!["abc@def"]);
    }
    {
        let src = r"^[a-zA-Z0-9_\.\+\-]+@[a-zA-Z0-9_\.]+[a-zA-Z]+$";
        let vm = Nfa::new(src).unwrap();

        assert_eq!(vm.captures("abc@example.com"), vec!["abc@example.com"]);
        assert_eq!(
            vm.captures("abc+123@me.example.com"),
            vec!["abc+123@me.example.com"]
        );
        assert_eq!(vm.captures("abc@example"), vec!["abc@example"]);
        assert_eq!(vm.captures("abc@example.123"), Vec::<&str>::new());
        assert_eq!(vm.captures("abc@def@example.com"), Vec::<&str>::new());
    }
}
