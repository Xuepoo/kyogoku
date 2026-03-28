label chapter_1:
    scene bg classroom_morning
    with fade

    $ narrator = Character(None)
    $ misaki = Character("美咲", color="#ff69b4")
    $ kenta = Character("健太", color="#4169e1")
    $ yukino = Character("雪乃", color="#87ceeb")
    $ teacher = Character("田中先生", color="#32cd32")
    $ mother = Character("母", color="#ffa500")

    narrator "朝の光が教室に差し込む中、私は窓際の席に座っていた。"

    show kenta normal at right
    with dissolve

    kenta "おはよう、美咲"

    show misaki smile at left
    with dissolve

    misaki "おはよう。今日も早いね"

    kenta "まあな。昨日の宿題、終わった？"

    misaki "うん、なんとかね。数学が難しかったけど"

    kenta "だよな。俺も苦労したわ"

    play sound "sfx/chime.ogg"
    narrator "チャイムが鳴り、担任の田中先生が教室に入ってきた。"

    hide kenta
    show teacher normal at center
    with dissolve

    teacher "はい、席についてー。今日は転校生を紹介します"

    narrator "クラス中がざわめいた。転校生なんて珍しい。"

    teacher "入って"

    play sound "sfx/door.ogg"
    narrator "ドアが開き、一人の少女が入ってきた。長い黒髪に、透き通るような白い肌。どこか儚げな雰囲気を纏っている。"

    show yukino shy at right
    with dissolve

    teacher "自己紹介をお願いします"

    yukino "……白石雪乃です。よろしくお願いします"

    narrator "短い自己紹介だったが、その声には不思議な響きがあった。"

    teacher "白石さんの席は……あそこ、窓際の空いてる席ね"

    narrator "先生が指さしたのは、私の隣の席だった。"

    hide teacher
    show misaki smile at left
    with dissolve

    misaki "よろしくね、白石さん"

    yukino "……よろしく"

    narrator "彼女は小さく頷いただけで、すぐに窓の外を見つめ始めた。"

    show kenta normal at center
    with dissolve

    kenta "なんか、変わった子だな"

    misaki "そうかな？ただ緊張してるだけかも"

    kenta "かもな"

    narrator "その日から、私の日常は少しずつ変わり始めた。"

    scene bg rooftop
    with fade

    narrator "昼休み、私は雪乃に声をかけた。"

    show misaki smile at left
    show yukino shy at right
    with dissolve

    misaki "ねえ、一緒にお昼食べない？"

    yukino "……いいの？"

    misaki "もちろん。友達になりたいから"

    narrator "彼女の目が少し潤んだように見えた。"

    show yukino happy
    yukino "ありがとう……"

    misaki "どういたしまして。さ、行こう"

    narrator "私たちは屋上へ向かった。"

    misaki "いい天気だね"

    yukino "うん……"

    misaki "前の学校はどんなところだったの？"

    show yukino sad
    yukino "……普通の学校。でも、あまり友達はいなかった"

    misaki "そうなんだ。でも、ここでは私がいるから大丈夫だよ"

    yukino "美咲さんは……優しいね"

    misaki "そんなことないよ。普通だよ"

    yukino "普通が、一番難しいのに"

    narrator "その言葉の意味を、私はまだ理解できなかった。"

    scene bg street_evening
    with fade

    narrator "放課後、私は雪乃と一緒に帰ることになった。"

    show misaki normal at left
    show yukino normal at right
    with dissolve

    misaki "家、どこなの？"

    yukino "駅の近くのマンション"

    misaki "じゃあ、途中まで一緒だね"

    yukino "うん"

    narrator "二人で歩く帰り道。秋の風が心地よい。"

    misaki "ねえ、雪乃"

    yukino "なに？"

    misaki "なんでこっちに転校してきたの？"

    show yukino sad
    yukino "……親の仕事の都合"

    misaki "そうなんだ"

    narrator "何か言いづらそうな雰囲気を感じて、それ以上は聞かなかった。"

    misaki "また明日ね"

    show yukino smile
    yukino "うん。また明日"

    narrator "彼女の笑顔は、どこか寂しそうだった。"

    scene bg home_evening
    with fade

    narrator "家に帰ると、母が夕食の準備をしていた。"

    show mother normal at center
    with dissolve

    mother "おかえり。今日はどうだった？"

    show misaki smile at left
    misaki "転校生が来たんだ。白石雪乃って子"

    mother "へえ、仲良くなれそう？"

    misaki "うん、多分ね"

    mother "よかったじゃない"

    narrator "夕食を食べながら、私は雪乃のことを考えていた。彼女の寂しそうな目が、どうしても頭から離れなかった。"

    return
