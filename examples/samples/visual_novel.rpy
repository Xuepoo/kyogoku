# Ren'Py Visual Novel Sample

label start:
    scene bg classroom with fade
    "新しい学期が始まった。"
    
    "転校してきたばかりの僕は、まだこの学校に慣れていない。"
    
    show sakura smile at center
    sakura "おはよう！あなたが新しい転校生？"
    
    menu:
        "はい、よろしく":
            sakura "よろしくね！私は桜。一緒に頑張ろう！"
            $ affection += 1
        
        "まあ、そうだけど...":
            sakura "そっか、緊張してるの？大丈夫だよ！"
    
    "彼女の笑顔を見て、少し緊張がほぐれた。"
    
    sakura """
    ねえ、放課後一緒に帰らない？
    この辺りを案内してあげるよ！
    """
    
    hide sakura
    scene bg hallway with dissolve
    
    "教室のチャイムが鳴った。"
    "新しい学校生活が、今始まろうとしている。"
    
    return
