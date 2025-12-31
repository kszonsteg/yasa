import botbowl.web.server as server
from botbowl import register_bot
from yasa import YasaMCTS, YasaRandom

# from bots import DrefsanteBot
# register_bot("Drefsante", DrefsanteBot)

register_bot("yasa_mcts", YasaMCTS)
register_bot("yasa_random", YasaRandom)

if __name__ == "__main__":
    server.start_server(host="localhost", debug=True, use_reloader=False, port=1234)
