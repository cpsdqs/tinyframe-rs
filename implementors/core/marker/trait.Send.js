(function() {var implementors = {};
implementors["tiny_frame"] = [{text:"impl&lt;ID, Type&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> for <a class=\"struct\" href=\"tiny_frame/struct.Msg.html\" title=\"struct tiny_frame::Msg\">Msg</a>&lt;ID, Type&gt; <span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;ID: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a>,<br>&nbsp;&nbsp;&nbsp;&nbsp;Type: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a>,&nbsp;</span>",synthetic:true,types:["tiny_frame::Msg"]},{text:"impl&lt;ID&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> for <a class=\"struct\" href=\"tiny_frame/struct.MsgEncoder.html\" title=\"struct tiny_frame::MsgEncoder\">MsgEncoder</a>&lt;ID&gt; <span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;ID: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a>,&nbsp;</span>",synthetic:true,types:["tiny_frame::MsgEncoder"]},{text:"impl&lt;ID, Len, Type, Cksum&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> for <a class=\"struct\" href=\"tiny_frame/struct.MsgDecoder.html\" title=\"struct tiny_frame::MsgDecoder\">MsgDecoder</a>&lt;ID, Len, Type, Cksum&gt; <span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;ID: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a>,<br>&nbsp;&nbsp;&nbsp;&nbsp;Len: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a>,<br>&nbsp;&nbsp;&nbsp;&nbsp;Type: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a>,<br>&nbsp;&nbsp;&nbsp;&nbsp;&lt;Cksum as <a class=\"trait\" href=\"tiny_frame/checksum/trait.Checksum.html\" title=\"trait tiny_frame::checksum::Checksum\">Checksum</a>&gt;::<a class=\"type\" href=\"tiny_frame/checksum/trait.Checksum.html#associatedtype.Output\" title=\"type tiny_frame::checksum::Checksum::Output\">Output</a>: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a>,&nbsp;</span>",synthetic:true,types:["tiny_frame::MsgDecoder"]},{text:"impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> for <a class=\"enum\" href=\"tiny_frame/checksum/enum.NoCheck.html\" title=\"enum tiny_frame::checksum::NoCheck\">NoCheck</a>",synthetic:true,types:["tiny_frame::checksum::NoCheck"]},{text:"impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> for <a class=\"enum\" href=\"tiny_frame/checksum/enum.XorSum.html\" title=\"enum tiny_frame::checksum::XorSum\">XorSum</a>",synthetic:true,types:["tiny_frame::checksum::XorSum"]},{text:"impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> for <a class=\"enum\" href=\"tiny_frame/checksum/enum.Crc16Sum.html\" title=\"enum tiny_frame::checksum::Crc16Sum\">Crc16Sum</a>",synthetic:true,types:["tiny_frame::checksum::Crc16Sum"]},{text:"impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> for <a class=\"enum\" href=\"tiny_frame/checksum/enum.Crc32Sum.html\" title=\"enum tiny_frame::checksum::Crc32Sum\">Crc32Sum</a>",synthetic:true,types:["tiny_frame::checksum::Crc32Sum"]},];

            if (window.register_implementors) {
                window.register_implementors(implementors);
            } else {
                window.pending_implementors = implementors;
            }
        
})()
