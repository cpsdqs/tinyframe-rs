use super::*;

fn loopback_tf<T>() -> TinyFrame<T, u8, u8>
        where T: BufferWritable + GenericNumber {
    let mut tf: TinyFrame<T, u8, u8> = TinyFrame::new(Peer::Master);

    tf.write = Some(Box::new(|tf, buf| {
        println!("frame: {:?}", buf);
        tf.accept(&Vec::from(buf));
    }));

    tf
}

macro_rules! assert_callback_calls {
    ($name:expr, $calls:expr, $count:expr) => {
        if unsafe { $calls } != $count {
            panic!("{} was called an incorrect number of times: {} (should be {})", $name, unsafe { $calls }, $count);
        }
    }
}

#[test]
fn basic_test() {
    let mut tf = loopback_tf::<u8>();

    #[allow(non_upper_case_globals)]
    static mut first_msg: bool = true;

    #[allow(non_upper_case_globals)]
    static mut generic_calls: u32 = 0;

    let _listener = tf.add_generic_listener(Box::new(|_, msg| {
        println!("Generic listener! Message: {}", String::from_utf8_lossy(&msg.data[..]));

        if unsafe { first_msg } {
            assert_eq!(&msg.data[..], b"Hello TinyFrame");
            unsafe { first_msg = false };
        }

        unsafe { generic_calls += 1 };

        ListenerResult::Stay
    }));

    tf.send(Msg::new(0, b"Hello TinyFrame"));

    #[allow(non_upper_case_globals)]
    static mut query_calls: u32 = 0;

    tf.query(Msg::new(0, b"Query message"), Box::new(|_, msg| {
        println!("Query result: {}", String::from_utf8_lossy(&msg.data[..]));
        unsafe { query_calls += 1 };
        ListenerResult::Close
    }), None);

    assert_callback_calls!("Generic listener", generic_calls, 2);
    assert_callback_calls!("Query listener", query_calls, 1);
}

#[test]
fn type_listeners() {
    let mut tf = loopback_tf::<u8>();

    #[allow(non_upper_case_globals)]
    static mut type1_calls: u32 = 0;

    #[allow(non_upper_case_globals)]
    static mut type2_calls: u32 = 0;

    let _listener = tf.add_type_listener(1, Box::new(|_, msg| {
        println!("Type 1 message: {}", String::from_utf8_lossy(&msg.data[..]));
        unsafe { type1_calls += 1 };
        ListenerResult::Stay
    }));

    tf.send(Msg::new(1, b"Type 1 message"));

    let _listener1 = tf.add_type_listener(2, Box::new(|_, msg| {
        println!("Type 2 message: {}", String::from_utf8_lossy(&msg.data[..]));
        unsafe { type2_calls += 1 };
        ListenerResult::Stay
    }));

    tf.send(Msg::new(2, b"Type 2 message"));

    assert_callback_calls!("Type listener 1", type1_calls, 1);
    assert_callback_calls!("Type listener 2", type2_calls, 1);
}

#[test]
fn id_timeouts() {
    let mut tf = loopback_tf::<u8>();

    #[allow(non_upper_case_globals)]
    static mut id9_calls: u32 = 0;

    #[allow(non_upper_case_globals)]
    static mut id10_calls: u32 = 0;

    let _listener9 = tf.add_id_listener(128, Box::new(|_, _| {
        unsafe { id9_calls += 1 };
        ListenerResult::Stay
    }), Some(9));

    let _listener10 = tf.add_id_listener(128, Box::new(|_, _| {
        unsafe { id10_calls += 1 };
        ListenerResult::Stay
    }), Some(10));

    for _ in 0..9 {
        tf.tick();
    }

    tf.send(Msg::new(0, b"Message"));

    assert_callback_calls!("ID listener with timeout 9", id9_calls, 0);
    assert_callback_calls!("ID listener with timeout 10", id10_calls, 1);
}

#[test]
fn compare_with_c() {
    // byte strings from the C implementation

    {
        let mut tf: TinyFrame<u16, u8, u8> = TinyFrame::new(Peer::Master);
        tf.cksum = Checksum::Crc16;
        tf.sof_byte = Some(0x01);

        tf.write = Some(Box::new(|_tf, buf| {
            assert_eq!(buf, [1, 128, 0, 16, 34, 217, 153, 72, 101, 108, 108, 111, 32, 84, 105, 110, 121, 70, 114, 97, 109, 101, 0, 48, 44]);
        }));

        tf.send(Msg::new(34, b"Hello TinyFrame\0"));
    }

    {
        let mut tf: TinyFrame<u32, u32, u32> = TinyFrame::new(Peer::Master);
        tf.cksum = Checksum::Crc32;
        tf.sof_byte = Some(0x05);

        tf.write = Some(Box::new(|_tf, buf| {
            assert_eq!(buf, [5, 128, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 114, 156, 154, 113]);
        }));

        tf.send(Msg::new(0, &[]));

        tf.write = Some(Box::new(|_tf, buf| {
            // Rust doesn't implement PartialEq for [u8; 49]
            let mut comp_buf: Vec<u8> = Vec::with_capacity(49);
            for b in [5, 128, 0, 0, 1, 0, 0, 0, 28, 0, 0, 0, 51, 127, 39, 149, 167, 76, 111, 114, 101, 109, 32, 105, 112, 115, 117, 109, 32, 100, 111, 108, 111, 114, 32, 115, 105, 116, 32, 97, 109, 101, 116, 46, 0, 183, 134, 8, 209].iter() {
                comp_buf.push(*b);
            }
            assert_eq!(Vec::from(buf), comp_buf);
        }));

        tf.send(Msg::new(51, b"Lorem ipsum dolor sit amet.\0"));
    }
}

#[test]
fn multipart() {
    const ROMEO: &str = "THE TRAGEDY OF ROMEO AND JULIET

by William Shakespeare

Dramatis Personae

  Chorus.

  Escalus, Prince of Verona.

  Paris, a young Count, kinsman to the Prince.

  Montague, heads of two houses at variance with each other.

  Capulet, heads of two houses at variance with each other.

  An old Man, of the Capulet family.

  Romeo, son to Montague.

  Tybalt, nephew to Lady Capulet.

  Mercutio, kinsman to the Prince and friend to Romeo.

  Benvolio, nephew to Montague, and friend to Romeo

  Tybalt, nephew to Lady Capulet.

  Friar Laurence, Franciscan.

  Friar John, Franciscan.

  Balthasar, servant to Romeo.

  Abram, servant to Montague.

  Sampson, servant to Capulet.

  Gregory, servant to Capulet.

  Peter, servant to Juliet's nurse.

  An Apothecary.

  Three Musicians.

  An Officer.

  Lady Montague, wife to Montague.

  Lady Capulet, wife to Capulet.

  Juliet, daughter to Capulet.

  Nurse to Juliet.

  Citizens of Verona; Gentlemen and Gentlewomen of both houses;
    Maskers, Torchbearers, Pages, Guards, Watchmen, Servants, and
    Attendants.

                            SCENE.--Verona; Mantua.

                        THE PROLOGUE

                        Enter Chorus.

  Chor. Two households, both alike in dignity,
    In fair Verona, where we lay our scene,
    From ancient grudge break to new mutiny,
    Where civil blood makes civil hands unclean.
    From forth the fatal loins of these two foes
    A pair of star-cross'd lovers take their life;
    Whose misadventur'd piteous overthrows
    Doth with their death bury their parents' strife.
    The fearful passage of their death-mark'd love,
    And the continuance of their parents' rage,
    Which, but their children's end, naught could remove,
    Is now the two hours' traffic of our stage;
    The which if you with patient ears attend,
    What here shall miss, our toil shall strive to mend.
                                                         [Exit.]

ACT I. Scene I.
Verona. A public place.

Enter Sampson and Gregory (with swords and bucklers) of the house
of Capulet.

  Samp. Gregory, on my word, we'll not carry coals.

  Greg. No, for then we should be colliers.

  Samp. I mean, an we be in choler, we'll draw.

  Greg. Ay, while you live, draw your neck out of collar.

  Samp. I strike quickly, being moved.

  Greg. But thou art not quickly moved to strike.

  Samp. A dog of the house of Montague moves me.

  Greg. To move is to stir, and to be valiant is to stand.
    Therefore, if thou art moved, thou runn'st away.

  Samp. A dog of that house shall move me to stand. I will take
    the wall of any man or maid of Montague's.

  Greg. That shows thee a weak slave; for the weakest goes to the
    wall.

  Samp. 'Tis true; and therefore women, being the weaker vessels,
    are ever thrust to the wall. Therefore I will push Montague's men
    from the wall and thrust his maids to the wall.

  Greg. The quarrel is between our masters and us their men.

  Samp. 'Tis all one. I will show myself a tyrant. When I have
    fought with the men, I will be cruel with the maids- I will cut off
    their heads.

  Greg. The heads of the maids?

  Samp. Ay, the heads of the maids, or their maidenheads.
    Take it in what sense thou wilt.

  Greg. They must take it in sense that feel it.

  Samp. Me they shall feel while I am able to stand; and 'tis known I
    am a pretty piece of flesh.

  Greg. 'Tis well thou art not fish; if thou hadst, thou hadst
    been poor-John. Draw thy tool! Here comes two of the house of
    Montagues.

           Enter two other Servingmen [Abram and Balthasar].

  Samp. My naked weapon is out. Quarrel! I will back thee.

  Greg. How? turn thy back and run?

  Samp. Fear me not.

  Greg. No, marry. I fear thee!

  Samp. Let us take the law of our sides; let them begin.

  Greg. I will frown as I pass by, and let them take it as they list.

  Samp. Nay, as they dare. I will bite my thumb at them; which is
    disgrace to them, if they bear it.

  Abr. Do you bite your thumb at us, sir?

  Samp. I do bite my thumb, sir.

  Abr. Do you bite your thumb at us, sir?

  Samp. [aside to Gregory] Is the law of our side if I say ay?

  Greg. [aside to Sampson] No.

  Samp. No, sir, I do not bite my thumb at you, sir; but I bite my
    thumb, sir.

  Greg. Do you quarrel, sir?

  Abr. Quarrel, sir? No, sir.

  Samp. But if you do, sir, am for you. I serve as good a man as
    you.

  Abr. No better.

  Samp. Well, sir.

                        Enter Benvolio.

  Greg. [aside to Sampson] Say 'better.' Here comes one of my
    master's kinsmen.

  Samp. Yes, better, sir.

  Abr. You lie.

  Samp. Draw, if you be men. Gregory, remember thy swashing blow.
                                                     They fight.

  Ben. Part, fools! [Beats down their swords.]
    Put up your swords. You know not what you do.

                          Enter Tybalt.

  Tyb. What, art thou drawn among these heartless hinds?
    Turn thee Benvolio! look upon thy death.

  Ben. I do but keep the peace. Put up thy sword,
    Or manage it to part these men with me.

  Tyb. What, drawn, and talk of peace? I hate the word
    As I hate hell, all Montagues, and thee.
    Have at thee, coward!                            They fight.

     Enter an officer, and three or four Citizens with clubs or
                          partisans.

  Officer. Clubs, bills, and partisans! Strike! beat them down!

  Citizens. Down with the Capulets! Down with the Montagues!

           Enter Old Capulet in his gown, and his Wife.

  Cap. What noise is this? Give me my long sword, ho!

  Wife. A crutch, a crutch! Why call you for a sword?

  Cap. My sword, I say! Old Montague is come
    And flourishes his blade in spite of me.

                 Enter Old Montague and his Wife.

  Mon. Thou villain Capulet!- Hold me not, let me go.

  M. Wife. Thou shalt not stir one foot to seek a foe.

                Enter Prince Escalus, with his Train.

END OF FILE\n";

    let mut tf = loopback_tf::<u16>();

    #[allow(non_upper_case_globals)]
    static mut generic_calls: u32 = 0;

    let _l = tf.add_generic_listener(Box::new(|_, msg| {
        assert_eq!(String::from_utf8_lossy(&msg.data[..]), ROMEO);
        unsafe { generic_calls += 1 };
        ListenerResult::Close
    }));

    tf.send(Msg::new(0, ROMEO.as_bytes()));

    assert_callback_calls!("Generic listener", generic_calls, 1);
}
